use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// RIFF (AVI and WAV, including RF64 / BW64) container demuxer.
pub struct RiffDemuxer;

impl RiffDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        let magic = &data[0..4];
        let is_riff_family = magic == b"RIFF" || magic == b"RIFX" || magic == b"RF64" || magic == b"BW64";

        if !is_riff_family {
            return Err(MediaInfoError::InvalidData("Not a valid RIFF/RF64 file".to_string()));
        }

        let form_type = &data[8..12];
        let mut report = MediaReport::new();
        report.general.file_size = data.len() as u64;

        if form_type == b"WAVE" {
            report.general.format = ContainerFormat::WAV;
            Self::parse_wav(data, &mut report)?;
        } else if form_type == b"AVI " || form_type == b"AVIX" {
            report.general.format = ContainerFormat::AVI;
            Self::parse_avi(data, &mut report)?;
        }

        Ok(report)
    }

    fn parse_wav(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let mut offset = 12;
        let mut audio_track = AudioTrack::default();
        audio_track.format = AudioCodec::PCM;
        audio_track.format_info = Some("Pulse Code Modulation".to_string());
        let mut data_chunk_size = 0u64;

        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;

            let payload_offset = offset + 8;
            let payload_size = if payload_offset + chunk_size <= data.len() {
                chunk_size
            } else {
                data.len().saturating_sub(payload_offset)
            };

            let payload = &data[payload_offset..payload_offset + payload_size];

            if chunk_id == b"ds64" && payload.len() >= 16 {
                // RF64 64-bit size chunk
                let ds_data_size = u64::from_le_bytes([
                    payload[8], payload[9], payload[10], payload[11],
                    payload[12], payload[13], payload[14], payload[15],
                ]);
                if ds_data_size > 0 {
                    data_chunk_size = ds_data_size;
                }
            } else if chunk_id == b"fmt " && payload.len() >= 16 {
                let format_tag = u16::from_le_bytes([payload[0], payload[1]]);
                let channels = u16::from_le_bytes([payload[2], payload[3]]) as u32;
                let sample_rate = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let byte_rate = u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
                let _block_align = u16::from_le_bytes([payload[12], payload[13]]);
                let bit_depth = u16::from_le_bytes([payload[14], payload[15]]) as u8;

                audio_track.channels = channels;
                audio_track.sampling_rate = sample_rate;
                audio_track.bit_rate = Some(byte_rate as u64 * 8);
                audio_track.bit_depth = Some(bit_depth);
                audio_track.compression_mode = Some(if format_tag == 1 || format_tag == 3 || format_tag == 0xFFFE {
                    "Lossless".to_string()
                } else {
                    "Lossy".to_string()
                });

                audio_track.channel_layout = match channels {
                    1 => Some(AudioChannelLayout::Mono),
                    2 => Some(AudioChannelLayout::Stereo),
                    6 => Some(AudioChannelLayout::Surround5_1),
                    8 => Some(AudioChannelLayout::Surround7_1),
                    _ => Some(AudioChannelLayout::Stereo),
                };
            } else if chunk_id == b"data" {
                if data_chunk_size == 0 {
                    data_chunk_size = chunk_size as u64;
                }
            }

            if chunk_id == b"data" {
                // data chunk is huge, skip to next or finish
                break;
            }

            // Word-aligned chunk padding
            offset = payload_offset + chunk_size + (chunk_size % 2);
        }

        if let Some(bitrate) = audio_track.bit_rate {
            if bitrate > 0 && data_chunk_size > 0 {
                let duration_ms = ((data_chunk_size * 8) as f64 / bitrate as f64) * 1000.0;
                audio_track.duration_ms = Some(duration_ms);
                report.general.duration_ms = Some(duration_ms);
                report.general.overall_bitrate = Some(bitrate);
            }
        }

        report.audios.push(audio_track);
        Ok(())
    }

    fn parse_avi(data: &[u8], report: &mut MediaReport) -> Result<()> {
        let mut offset = 12;
        let mut video_track = VideoTrack::default();
        let mut audio_track = AudioTrack::default();
        let mut has_video = false;
        let mut has_audio = false;

        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;

            let payload_offset = offset + 8;
            if payload_offset + chunk_size > data.len() {
                break;
            }

            let payload = &data[payload_offset..payload_offset + chunk_size];

            if chunk_id == b"avih" && payload.len() >= 40 {
                let microsec_per_frame = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let total_frames = u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);
                let width = u32::from_le_bytes([payload[32], payload[33], payload[34], payload[35]]);
                let height = u32::from_le_bytes([payload[36], payload[37], payload[38], payload[39]]);

                if microsec_per_frame > 0 {
                    let fps = 1_000_000.0 / microsec_per_frame as f64;
                    video_track.frame_rate = Some(fps);
                    if total_frames > 0 {
                        let duration_ms = (total_frames as f64 / fps) * 1000.0;
                        report.general.duration_ms = Some(duration_ms);
                        video_track.duration_ms = Some(duration_ms);
                    }
                }

                video_track.width = width;
                video_track.height = height;
                has_video = true;
            } else if chunk_id == b"strh" && payload.len() >= 12 {
                let stream_type = &payload[0..4];
                let handler = &payload[4..8];
                let handler_str = String::from_utf8_lossy(handler).to_string();

                if stream_type == b"vids" {
                    video_track.codec_id = Some(handler_str.clone());
                    video_track.format = match handler {
                        b"H264" | b"h264" | b"X264" | b"x264" | b"AVC1" | b"avc1" => VideoCodec::AVC,
                        b"HEVC" | b"hevc" | b"H265" | b"h265" => VideoCodec::HEVC,
                        b"XVID" | b"xvid" | b"DIVX" | b"divx" | b"DX50" => VideoCodec::MPEG4Visual,
                        _ => VideoCodec::Other(handler_str),
                    };
                    has_video = true;
                } else if stream_type == b"auds" {
                    audio_track.codec_id = Some(handler_str);
                    has_audio = true;
                }
            } else if chunk_id == b"strf" && payload.len() >= 16 {
                if has_video && video_track.format == VideoCodec::Other("Unknown".to_string()) {
                    let bi_compression = &payload[16..20.min(payload.len())];
                    if bi_compression.len() == 4 {
                        let comp_str = String::from_utf8_lossy(bi_compression).to_string();
                        video_track.codec_id = Some(comp_str.clone());
                        video_track.format = VideoCodec::Other(comp_str);
                    }
                }
            }

            offset = payload_offset + chunk_size + (chunk_size % 2);
            if offset > 1024 * 1024 && (has_video || has_audio) {
                break;
            }
        }

        if has_video {
            report.videos.push(video_track);
        }
        if has_audio {
            report.audios.push(audio_track);
        }

        Ok(())
    }
}
