use mediainfo_audio::OpusHead;
use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::VorbisComments;

/// Ogg container demuxer (Vorbis, Opus, Theora).
pub struct OggDemuxer;

impl OggDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 27 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 27,
                actual: data.len(),
            });
        }

        if !data.starts_with(b"OggS") {
            return Err(MediaInfoError::InvalidData("Not a valid Ogg bitstream".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::Ogg;
        report.general.file_size = data.len() as u64;

        let mut offset = 0;
        let mut audio_track = AudioTrack::default();

        while offset + 27 <= data.len() {
            if &data[offset..offset + 4] != b"OggS" {
                offset += 1;
                continue;
            }

            let num_segments = data[offset + 26] as usize;
            let header_len = 27 + num_segments;

            if offset + header_len > data.len() {
                break;
            }

            let seg_table = &data[offset + 27..offset + header_len];
            let mut page_payload_size = 0usize;
            for &seg in seg_table {
                page_payload_size += seg as usize;
            }

            let payload_offset = offset + header_len;
            if payload_offset + page_payload_size > data.len() {
                break;
            }

            let page_payload = &data[payload_offset..payload_offset + page_payload_size];

            if page_payload.starts_with(b"OpusHead") {
                if let Ok(opus) = OpusHead::parse(page_payload) {
                    audio_track.format = AudioCodec::Opus;
                    audio_track.format_info = Some("Opus Audio".to_string());
                    audio_track.channels = opus.channels;
                    audio_track.channel_layout = Some(opus.channel_layout);
                    audio_track.sampling_rate = opus.output_sample_rate;
                }
            } else if page_payload.starts_with(b"OpusTags") {
                if let Ok(tags) = VorbisComments::parse(&page_payload[8..]) {
                    report.general.title = tags.title;
                    report.general.artist = tags.artist;
                    report.general.album = tags.album;
                    report.general.recorded_date = tags.date;
                    report.general.genre = tags.genre;
                }
            } else if page_payload.starts_with(b"\x01vorbis") {
                audio_track.format = AudioCodec::Vorbis;
                audio_track.format_info = Some("Vorbis Audio".to_string());
                if page_payload.len() >= 29 {
                    let channels = page_payload[11] as u32;
                    let sample_rate = u32::from_le_bytes([
                        page_payload[12],
                        page_payload[13],
                        page_payload[14],
                        page_payload[15],
                    ]);
                    let nominal_bitrate = u32::from_le_bytes([
                        page_payload[20],
                        page_payload[21],
                        page_payload[22],
                        page_payload[23],
                    ]);

                    audio_track.channels = channels;
                    audio_track.sampling_rate = sample_rate;
                    if nominal_bitrate > 0 {
                        audio_track.bit_rate = Some(nominal_bitrate as u64);
                    }
                    audio_track.channel_layout = match channels {
                        1 => Some(AudioChannelLayout::Mono),
                        2 => Some(AudioChannelLayout::Stereo),
                        6 => Some(AudioChannelLayout::Surround5_1),
                        _ => Some(AudioChannelLayout::Stereo),
                    };
                }
            } else if page_payload.starts_with(b"\x03vorbis") {
                if let Ok(tags) = VorbisComments::parse(&page_payload[7..]) {
                    report.general.title = tags.title;
                    report.general.artist = tags.artist;
                    report.general.album = tags.album;
                    report.general.recorded_date = tags.date;
                    report.general.genre = tags.genre;
                }
            }

            offset = payload_offset + page_payload_size;
            // Stop scanning after initial header pages
            if offset > 65536 && audio_track.format != AudioCodec::Other("Unknown".to_string()) {
                break;
            }
        }

        // Scan backwards for last OggS page to find total granule_position (total samples)
        let tail_start = data.len().saturating_sub(65536);
        let tail = &data[tail_start..];
        let mut last_granule_pos = None;
        for i in (0..tail.len().saturating_sub(27)).rev() {
            if &tail[i..i + 4] == b"OggS" {
                let granule = u64::from_le_bytes([
                    tail[i + 6], tail[i + 7], tail[i + 8], tail[i + 9],
                    tail[i + 10], tail[i + 11], tail[i + 12], tail[i + 13],
                ]);
                if granule > 0 && granule != u64::MAX {
                    last_granule_pos = Some(granule);
                    break;
                }
            }
        }

        if let Some(granule) = last_granule_pos {
            let sample_rate = if audio_track.sampling_rate > 0 { audio_track.sampling_rate } else { 48000 };
            let dur_ms = (granule as f64 / sample_rate as f64) * 1000.0;
            if dur_ms > 0.0 {
                report.general.duration_ms = Some(dur_ms);
                audio_track.duration_ms = Some(dur_ms);
                let br = ((data.len() as u64 * 8) as f64 / (dur_ms / 1000.0)) as u64;
                if audio_track.bit_rate.is_none() {
                    audio_track.bit_rate = Some(br);
                }
                report.general.overall_bitrate = Some(br);
            }
        }

        if audio_track.format != AudioCodec::Other("Unknown".to_string()) {
            report.audios.push(audio_track);
        }

        Ok(report)
    }
}
