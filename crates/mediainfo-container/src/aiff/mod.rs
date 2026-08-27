use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::Id3v2Tag;

/// AIFF / AIFF-C Container Demuxer.
pub struct AiffDemuxer;

pub const FORM_MAGIC: [u8; 4] = [b'F', b'O', b'R', b'M'];

impl AiffDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        if !data.starts_with(&FORM_MAGIC) {
            return Err(MediaInfoError::InvalidData("Not a valid AIFF file".to_string()));
        }

        let form_type = &data[8..12];
        if form_type != b"AIFF" && form_type != b"AIFC" {
            return Err(MediaInfoError::InvalidData("Not an AIFF or AIFC container".to_string()));
        }

        let is_aifc = form_type == b"AIFC";

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::AIFF;
        report.general.file_size = data.len() as u64;
        if is_aifc {
            report.general.format_profile = Some("AIFF-C (Compressed AIFF)".to_string());
        }

        let mut audio_track = AudioTrack::default();
        audio_track.format = AudioCodec::PCM;
        audio_track.format_info = Some("Linear PCM".to_string());
        audio_track.compression_mode = Some("Lossless".to_string());

        let mut has_audio = false;
        let mut sample_rate_val = 44100u32;
        let mut sample_frames = 0u32;

        let mut offset = 12;
        while offset + 8 <= data.len() {
            let chunk_id = &data[offset..offset + 4];
            let chunk_size = u32::from_be_bytes([
                data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
            ]) as usize;
            offset += 8;

            if offset + chunk_size > data.len() {
                break;
            }

            let payload = &data[offset..offset + chunk_size];
            // AIFF chunks are padded to even bytes
            offset += chunk_size + (chunk_size % 2);

            match chunk_id {
                b"COMM" if payload.len() >= 18 => {
                    let channels = i16::from_be_bytes([payload[0], payload[1]]) as u32;
                    sample_frames = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
                    let bit_depth = i16::from_be_bytes([payload[6], payload[7]]) as u8;
                    sample_rate_val = Self::decode_ieee_extended(&payload[8..18]);

                    audio_track.channels = channels;
                    audio_track.sampling_rate = sample_rate_val;
                    audio_track.bit_depth = Some(bit_depth);
                    audio_track.channel_layout = match channels {
                        1 => Some(AudioChannelLayout::Mono),
                        2 => Some(AudioChannelLayout::Stereo),
                        6 => Some(AudioChannelLayout::Surround5_1),
                        _ => Some(AudioChannelLayout::Stereo),
                    };

                    if is_aifc && payload.len() >= 22 {
                        let comp_type = &payload[18..22];
                        match comp_type {
                            b"sowt" => {
                                audio_track.format_info = Some("Little-Endian Linear PCM".to_string());
                            }
                            b"fl32" | b"FL32" => {
                                audio_track.format_info = Some("32-bit Floating Point PCM".to_string());
                            }
                            b"fl64" | b"FL64" => {
                                audio_track.format_info = Some("64-bit Floating Point PCM".to_string());
                            }
                            b"alaw" | b"ALAW" => {
                                audio_track.format_info = Some("A-law".to_string());
                                audio_track.compression_mode = Some("Lossy".to_string());
                            }
                            b"ulaw" | b"ULAW" => {
                                audio_track.format_info = Some("mu-law".to_string());
                                audio_track.compression_mode = Some("Lossy".to_string());
                            }
                            _ => {
                                let comp_name = String::from_utf8_lossy(comp_type).to_string();
                                audio_track.format_info = Some(comp_name);
                            }
                        }
                    }
                    has_audio = true;
                }
                b"ID3 " => {
                    if let Ok(Some(id3)) = Id3v2Tag::parse(payload) {
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
                b"NAME" => {
                    report.general.title = Some(String::from_utf8_lossy(payload).trim_end_matches('\0').trim().to_string());
                }
                b"AUTH" => {
                    report.general.artist = Some(String::from_utf8_lossy(payload).trim_end_matches('\0').trim().to_string());
                }
                b"(c) " => {
                    report.general.extra.insert("Copyright".to_string(), String::from_utf8_lossy(payload).trim_end_matches('\0').trim().to_string());
                }
                _ => {}
            }
        }

        if sample_rate_val > 0 && sample_frames > 0 {
            let duration_ms = (sample_frames as f64 / sample_rate_val as f64) * 1000.0;
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

    /// Decodes an 80-bit IEEE 754 extended precision float to u32 sample rate.
    fn decode_ieee_extended(bytes: &[u8]) -> u32 {
        if bytes.len() < 10 {
            return 44100;
        }
        let expon = (((bytes[0] as u16 & 0x7F) << 8) | (bytes[1] as u16)) as i32;
        let hi_mant = u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as f64;
        let lo_mant = u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as f64;

        if expon == 0 && hi_mant == 0.0 && lo_mant == 0.0 {
            return 0;
        }

        let f = (hi_mant * 4294967296.0 + lo_mant) * 2.0f64.powi(expon - 16383 - 63);
        f.round() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aiff_demuxer() {
        let mut data = Vec::new();
        // FORM chunk
        data.extend_from_slice(&FORM_MAGIC);
        data.extend_from_slice(&38u32.to_be_bytes()); // total size
        data.extend_from_slice(b"AIFF");

        // COMM chunk
        data.extend_from_slice(b"COMM");
        data.extend_from_slice(&18u32.to_be_bytes());
        data.extend_from_slice(&2i16.to_be_bytes()); // 2 channels
        data.extend_from_slice(&44100u32.to_be_bytes()); // 44100 sample frames (1 sec)
        data.extend_from_slice(&16i16.to_be_bytes()); // 16-bit
        // 80-bit float for 44100.0: expon = 16383 + 15 = 16398 (0x400E), mantissa = 44100 << 48 = 0xAC440000_00000000
        data.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let report = AiffDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::AIFF);
        assert_eq!(report.general.duration_ms, Some(1000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].channels, 2);
        assert_eq!(report.audios[0].sampling_rate, 44100);
        assert_eq!(report.audios[0].bit_depth, Some(16));
    }
}
