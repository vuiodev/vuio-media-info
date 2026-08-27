use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::Id3v2Tag;

/// Direct Stream Digital (DSF & DSDIFF) Demuxer.
pub struct DsdDemuxer;

pub const DSF_MAGIC: [u8; 4] = [b'D', b'S', b'D', b' '];
pub const DSDIFF_MAGIC: [u8; 4] = [b'F', b'R', b'M', b'8'];

impl DsdDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        if data.starts_with(&DSF_MAGIC) {
            Self::parse_dsf(data)
        } else if data.starts_with(&DSDIFF_MAGIC) {
            Self::parse_dsdiff(data)
        } else {
            Err(MediaInfoError::InvalidData("Not a valid DSD file".to_string()))
        }
    }

    fn parse_dsf(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 28 + 52 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 80,
                actual: data.len(),
            });
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::DSF;
        report.general.file_size = data.len() as u64;

        // DSD chunk
        let metadata_offset = u64::from_le_bytes([
            data[20], data[21], data[22], data[23],
            data[24], data[25], data[26], data[27],
        ]) as usize;

        // fmt chunk (starts at offset 28)
        let fmt_chunk = &data[28..];
        if !fmt_chunk.starts_with(b"fmt ") || fmt_chunk.len() < 52 {
            return Err(MediaInfoError::InvalidData("Missing DSF fmt chunk".to_string()));
        }

        let channels = u32::from_le_bytes([fmt_chunk[24], fmt_chunk[25], fmt_chunk[26], fmt_chunk[27]]);
        let sample_rate = u32::from_le_bytes([fmt_chunk[28], fmt_chunk[29], fmt_chunk[30], fmt_chunk[31]]);
        let bits_per_sample = u32::from_le_bytes([fmt_chunk[32], fmt_chunk[33], fmt_chunk[34], fmt_chunk[35]]) as u8;
        let sample_count = u64::from_le_bytes([
            fmt_chunk[36], fmt_chunk[37], fmt_chunk[38], fmt_chunk[39],
            fmt_chunk[40], fmt_chunk[41], fmt_chunk[42], fmt_chunk[43],
        ]);

        let dsd_rate_name = match sample_rate {
            2822400 => "DSD64 (1-bit / 2.8224 MHz)",
            5644800 => "DSD128 (1-bit / 5.6448 MHz)",
            11289600 => "DSD256 (1-bit / 11.2896 MHz)",
            22579200 => "DSD512 (1-bit / 22.5792 MHz)",
            45158400 => "DSD1024 (1-bit / 45.1584 MHz)",
            _ => "DSD (1-bit)",
        };

        let mut a = AudioTrack::default();
        a.format = AudioCodec::DSD;
        a.format_info = Some(dsd_rate_name.to_string());
        a.format_profile = Some(dsd_rate_name.to_string());
        a.channels = channels;
        a.sampling_rate = sample_rate;
        a.bit_depth = Some(bits_per_sample);
        a.compression_mode = Some("Lossless".to_string());

        a.channel_layout = match channels {
            1 => Some(AudioChannelLayout::Mono),
            2 => Some(AudioChannelLayout::Stereo),
            6 => Some(AudioChannelLayout::Surround5_1),
            _ => Some(AudioChannelLayout::Stereo),
        };

        if sample_rate > 0 && sample_count > 0 {
            let duration_ms = (sample_count as f64 / sample_rate as f64) * 1000.0;
            a.duration_ms = Some(duration_ms);
            report.general.duration_ms = Some(duration_ms);

            let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
            a.bit_rate = Some(br);
            report.general.overall_bitrate = Some(br);
        }

        // Try parsing ID3v2 tags from metadata offset
        if metadata_offset > 0 && metadata_offset < data.len() {
            if let Ok(Some(id3)) = Id3v2Tag::parse(&data[metadata_offset..]) {
                report.general.title = id3.title;
                report.general.artist = id3.artist;
                report.general.album = id3.album;
                report.general.recorded_date = id3.date;
                report.general.genre = id3.genre;
                if id3.cover_data.is_some() {
                    report.general.cover_art_present = true;
                    report.general.cover_mime = id3.cover_mime;
                }
            }
        }

        report.audios.push(a);
        Ok(report)
    }

    fn parse_dsdiff(data: &[u8]) -> Result<MediaReport> {
        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::DSDIFF;
        report.general.file_size = data.len() as u64;

        let mut a = AudioTrack::default();
        a.format = AudioCodec::DSD;
        a.bit_depth = Some(1);
        a.compression_mode = Some("Lossless".to_string());

        let mut offset = 12; // Skip 'FRM8' + size (8) + 'DSD ' (4)
        while offset + 12 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u64::from_be_bytes([
                data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
                data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11],
            ]) as usize;
            offset += 12;

            if offset + chunk_size > data.len() {
                break;
            }
            let payload = &data[offset..offset + chunk_size];
            offset += chunk_size;

            if chunk_id == b"PROP" && payload.len() >= 4 && &payload[0..4] == b"SND " {
                let mut prop_offset = 4;
                while prop_offset + 12 <= payload.len() {
                    let sub_id = &payload[prop_offset..prop_offset + 4];
                    let sub_size = u64::from_be_bytes([
                        payload[prop_offset + 4], payload[prop_offset + 5], payload[prop_offset + 6], payload[prop_offset + 7],
                        payload[prop_offset + 8], payload[prop_offset + 9], payload[prop_offset + 10], payload[prop_offset + 11],
                    ]) as usize;
                    prop_offset += 12;

                    if prop_offset + sub_size > payload.len() {
                        break;
                    }
                    let sub_payload = &payload[prop_offset..prop_offset + sub_size];
                    prop_offset += sub_size;

                    match sub_id {
                        b"FSAM" if sub_payload.len() >= 4 => {
                            let sr = u32::from_be_bytes([sub_payload[0], sub_payload[1], sub_payload[2], sub_payload[3]]);
                            a.sampling_rate = sr;
                            let name = match sr {
                                2822400 => "DSD64 (1-bit / 2.8224 MHz)",
                                5644800 => "DSD128 (1-bit / 5.6448 MHz)",
                                11289600 => "DSD256 (1-bit / 11.2896 MHz)",
                                _ => "DSD (1-bit)",
                            };
                            a.format_info = Some(name.to_string());
                        }
                        b"CHNL" if sub_payload.len() >= 2 => {
                            let ch = u16::from_be_bytes([sub_payload[0], sub_payload[1]]) as u32;
                            a.channels = ch;
                            a.channel_layout = match ch {
                                1 => Some(AudioChannelLayout::Mono),
                                2 => Some(AudioChannelLayout::Stereo),
                                6 => Some(AudioChannelLayout::Surround5_1),
                                _ => Some(AudioChannelLayout::Stereo),
                            };
                        }
                        b"CMPR" if sub_payload.len() >= 4 => {
                            let comp = &sub_payload[0..4];
                            if comp == b"DST " {
                                a.compression_mode = Some("Lossless (Direct Stream Transfer)".to_string());
                                a.format_info = Some("DST Compressed DSD".to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if a.channels == 0 {
            a.channels = 2;
            a.channel_layout = Some(AudioChannelLayout::Stereo);
        }
        if a.sampling_rate == 0 {
            a.sampling_rate = 2822400;
        }

        report.audios.push(a);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsd_dsf_demuxer() {
        let mut data = Vec::new();
        // DSD chunk (28 bytes)
        data.extend_from_slice(&DSF_MAGIC);
        data.extend_from_slice(&28u64.to_le_bytes()); // chunk size
        data.extend_from_slice(&80u64.to_le_bytes()); // file size
        data.extend_from_slice(&0u64.to_le_bytes()); // metadata offset

        // fmt chunk (52 bytes)
        data.extend_from_slice(b"fmt ");
        data.extend_from_slice(&52u64.to_le_bytes()); // chunk size
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&0u32.to_le_bytes()); // format ID (raw DSD)
        data.extend_from_slice(&2u32.to_le_bytes()); // channel type (stereo)
        data.extend_from_slice(&2u32.to_le_bytes()); // channel count
        data.extend_from_slice(&2822400u32.to_le_bytes()); // 2.8224 MHz (DSD64)
        data.extend_from_slice(&1u32.to_le_bytes()); // 1 bit
        data.extend_from_slice(&28224000u64.to_le_bytes()); // 10 seconds of samples
        data.extend_from_slice(&4096u32.to_le_bytes()); // block size
        data.extend_from_slice(&0u32.to_le_bytes()); // reserved

        let report = DsdDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::DSF);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].sampling_rate, 2822400);
        assert_eq!(report.audios[0].bit_depth, Some(1));
    }
}
