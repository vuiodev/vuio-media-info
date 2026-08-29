use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// Apple CoreAudio Format (`.caf`) Demuxer.
pub struct CafDemuxer;

pub const CAF_MAGIC: [u8; 4] = [b'c', b'a', b'f', b'f'];

impl CafDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        if !data.starts_with(&CAF_MAGIC) {
            return Err(MediaInfoError::InvalidData(
                "Not a valid CAF file".to_string(),
            ));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::CAF;
        report.general.file_size = data.len() as u64;

        let mut audio_track = AudioTrack::default();
        let mut sample_rate = 44100.0f64;
        let mut valid_frames = 0u64;
        let mut has_audio = false;

        let mut offset = 8; // Skip 'caff' + version (2) + flags (2)

        while offset + 12 <= data.len() {
            let chunk_type = &data[offset..offset + 4];
            let chunk_size_raw = i64::from_be_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);
            offset += 12;

            let chunk_size = if chunk_size_raw < 0 {
                data.len().saturating_sub(offset)
            } else {
                chunk_size_raw as usize
            };

            let payload_end = (offset + chunk_size).min(data.len());
            let payload = &data[offset..payload_end];
            offset = payload_end;

            match chunk_type {
                b"desc" if payload.len() >= 32 => {
                    sample_rate = f64::from_be_bytes([
                        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                        payload[6], payload[7],
                    ]);
                    let format_id = &payload[8..12];
                    audio_track.codec_id =
                        Some(String::from_utf8_lossy(format_id).trim().to_string());
                    let format_flags =
                        u32::from_be_bytes([payload[12], payload[13], payload[14], payload[15]]);
                    let _bytes_per_packet =
                        u32::from_be_bytes([payload[16], payload[17], payload[18], payload[19]]);
                    let _frames_per_packet =
                        u32::from_be_bytes([payload[20], payload[21], payload[22], payload[23]]);
                    let channels =
                        u32::from_be_bytes([payload[24], payload[25], payload[26], payload[27]]);
                    let bits_per_channel =
                        u32::from_be_bytes([payload[28], payload[29], payload[30], payload[31]]);

                    audio_track.sampling_rate = sample_rate as u32;
                    audio_track.channels = channels;
                    if bits_per_channel > 0 {
                        audio_track.bit_depth = Some(bits_per_channel as u8);
                    }

                    audio_track.channel_layout = match channels {
                        1 => Some(AudioChannelLayout::Mono),
                        2 => Some(AudioChannelLayout::Stereo),
                        6 => Some(AudioChannelLayout::Surround5_1),
                        8 => Some(AudioChannelLayout::Surround7_1),
                        _ => Some(AudioChannelLayout::Stereo),
                    };

                    // Uncompressed formats have a bit rate fixed by the sample format.
                    if format_id == b"lpcm" && bits_per_channel > 0 && channels > 0 {
                        audio_track.bit_rate =
                            Some(sample_rate as u64 * channels as u64 * bits_per_channel as u64);
                        audio_track.bit_rate_mode = Some(BitrateMode::Constant);
                    }

                    match format_id {
                        b"lpcm" => {
                            audio_track.format = AudioCodec::PCM;
                            let is_float = (format_flags & 1) != 0;
                            audio_track.format_info = Some(if is_float {
                                "Linear PCM (IEEE Float)".to_string()
                            } else {
                                "Linear PCM".to_string()
                            });
                            audio_track.compression_mode = Some("Lossless".to_string());
                        }
                        b"alac" => {
                            audio_track.format = AudioCodec::ALAC;
                            audio_track.format_info =
                                Some("Apple Lossless Audio Codec".to_string());
                            audio_track.compression_mode = Some("Lossless".to_string());
                        }
                        b"aac " => {
                            audio_track.format = AudioCodec::AAC;
                            audio_track.format_info = Some("Advanced Audio Coding".to_string());
                            audio_track.compression_mode = Some("Lossy".to_string());
                        }
                        b"opus" => {
                            audio_track.format = AudioCodec::Opus;
                            audio_track.format_info = Some("Opus Audio".to_string());
                            audio_track.compression_mode = Some("Lossy".to_string());
                        }
                        b"flac" => {
                            audio_track.format = AudioCodec::FLAC;
                            audio_track.format_info = Some("Free Lossless Audio Codec".to_string());
                            audio_track.compression_mode = Some("Lossless".to_string());
                        }
                        _ => {
                            let fourcc_str = String::from_utf8_lossy(format_id).to_string();
                            audio_track.format = AudioCodec::Other(fourcc_str);
                        }
                    }
                    has_audio = true;
                }
                b"pakt" if payload.len() >= 16 => {
                    let _num_packets = u64::from_be_bytes([
                        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                        payload[6], payload[7],
                    ]);
                    valid_frames = u64::from_be_bytes([
                        payload[8],
                        payload[9],
                        payload[10],
                        payload[11],
                        payload[12],
                        payload[13],
                        payload[14],
                        payload[15],
                    ]);
                }
                b"info" if payload.len() >= 4 => {
                    let num_entries =
                        u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
                            as usize;
                    let mut str_offset = 4;
                    for _ in 0..num_entries {
                        if str_offset >= payload.len() {
                            break;
                        }
                        // Read null-terminated key
                        if let Some(key_end) = payload[str_offset..].iter().position(|&b| b == 0) {
                            let key =
                                String::from_utf8_lossy(&payload[str_offset..str_offset + key_end])
                                    .to_string();
                            str_offset += key_end + 1;
                            if str_offset >= payload.len() {
                                break;
                            }
                            if let Some(val_end) =
                                payload[str_offset..].iter().position(|&b| b == 0)
                            {
                                let val = String::from_utf8_lossy(
                                    &payload[str_offset..str_offset + val_end],
                                )
                                .to_string();
                                str_offset += val_end + 1;
                                match key.to_lowercase().as_str() {
                                    "title" => report.general.title = Some(val),
                                    "artist" => report.general.artist = Some(val),
                                    "album" => report.general.album = Some(val),
                                    "year" | "date" => report.general.recorded_date = Some(val),
                                    "genre" => report.general.genre = Some(val),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if sample_rate > 0.0 && valid_frames > 0 {
            let duration_ms = (valid_frames as f64 / sample_rate) * 1000.0;
            audio_track.duration_ms = Some(duration_ms);
            report.general.duration_ms = Some(duration_ms);

            let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
            audio_track.bit_rate = Some(br);
            report.general.overall_bitrate = Some(br);
        }

        if has_audio {
            report.audios.push(audio_track);
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caf_demuxer() {
        let mut data = Vec::new();
        // Header
        data.extend_from_slice(&CAF_MAGIC);
        data.extend_from_slice(&1u16.to_be_bytes()); // version 1
        data.extend_from_slice(&0u16.to_be_bytes()); // flags 0

        // 'desc' chunk
        data.extend_from_slice(b"desc");
        data.extend_from_slice(&32i64.to_be_bytes()); // size 32
        data.extend_from_slice(&96000.0f64.to_be_bytes()); // sample rate 96kHz
        data.extend_from_slice(b"lpcm"); // format
        data.extend_from_slice(&0u32.to_be_bytes()); // format flags
        data.extend_from_slice(&6u32.to_be_bytes()); // bytes per packet
        data.extend_from_slice(&1u32.to_be_bytes()); // frames per packet
        data.extend_from_slice(&2u32.to_be_bytes()); // channels: 2
        data.extend_from_slice(&24u32.to_be_bytes()); // bits: 24

        // 'pakt' chunk
        data.extend_from_slice(b"pakt");
        data.extend_from_slice(&24i64.to_be_bytes());
        data.extend_from_slice(&96000u64.to_be_bytes()); // packets: 96000
        data.extend_from_slice(&96000u64.to_be_bytes()); // valid frames: 96000 (1 sec)
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let report = CafDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::CAF);
        assert_eq!(report.general.duration_ms, Some(1000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].sampling_rate, 96000);
        assert_eq!(report.audios[0].bit_depth, Some(24));
    }
}
