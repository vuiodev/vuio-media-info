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

        if audio_track.format != AudioCodec::Other("Unknown".to_string()) {
            report.audios.push(audio_track);
        }

        Ok(report)
    }
}
