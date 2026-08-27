use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// MXF (Material Exchange Format - SMPTE 377M) Demuxer.
pub struct MxfDemuxer;

// SMPTE Universal Label (UL) prefix for MXF: 06 0E 2B 34
pub const MXF_SMPTE_PREFIX: [u8; 4] = [0x06, 0x0E, 0x2B, 0x34];

// Key types
const KEY_CDCI_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x28, 0x00];
const KEY_RGBA_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x29, 0x00];
const KEY_MPEG2_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x51, 0x00];
const KEY_WAVE_AUDIO_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x48, 0x00];
const KEY_GENERIC_SOUND_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x42, 0x00];
const KEY_IDENTIFICATION_SET: [u8; 4] = [0x01, 0x01, 0x30, 0x00];
const KEY_TIMELINE_TRACK: [u8; 4] = [0x01, 0x01, 0x3B, 0x00];
const KEY_SOURCE_CLIP: [u8; 4] = [0x01, 0x01, 0x11, 0x00];

impl MxfDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 16 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 16,
                actual: data.len(),
            });
        }

        if !data.starts_with(&MXF_SMPTE_PREFIX) {
            return Err(MediaInfoError::InvalidData(
                "Not a valid MXF file (missing SMPTE UL prefix)".to_string(),
            ));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::MXF;
        report.general.file_size = data.len() as u64;

        let mut offset = 0;
        let mut video_track = VideoTrack::default();
        let mut audio_track = AudioTrack::default();
        let mut has_video = false;
        let mut has_audio = false;
        let mut track_duration_edit_units: u64 = 0;
        let mut track_edit_rate: f64 = 0.0;

        let max_scan = data.len().min(4 * 1024 * 1024); // Scan up to 4MB of header metadata

        while offset + 16 < max_scan {
            if &data[offset..offset + 4] != &MXF_SMPTE_PREFIX {
                offset += 1;
                continue;
            }

            let key = &data[offset..offset + 16];
            offset += 16;

            let (val_len, len_bytes) = Self::parse_ber_length(&data[offset..])?;
            offset += len_bytes;

            if offset + val_len > data.len() {
                break;
            }

            let value = &data[offset..offset + val_len];
            offset += val_len;

            // Check Header Partition Pack: 06 0E 2B 34 02 05 01 01 0D 01 02 01 01
            if key[4..13] == [0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01] {
                let status_byte = key[13];
                let op_status = match status_byte {
                    0x02 => "Closed/Complete",
                    0x03 => "Closed/Header",
                    0x04 => "Open/Complete",
                    _ => "Open/Header",
                };
                report.general.format_profile = Some(format!("MXF ({})", op_status));
            }

            // Check sets by last 4 bytes of 16-byte key
            let key_tail = &key[12..16];

            if key_tail == KEY_CDCI_DESCRIPTOR
                || key_tail == KEY_RGBA_DESCRIPTOR
                || key_tail == KEY_MPEG2_DESCRIPTOR
            {
                Self::parse_picture_essence_descriptor(value, &mut video_track);
                has_video = true;
            } else if key_tail == KEY_WAVE_AUDIO_DESCRIPTOR
                || key_tail == KEY_GENERIC_SOUND_DESCRIPTOR
            {
                Self::parse_sound_essence_descriptor(value, &mut audio_track);
                has_audio = true;
            } else if key_tail == KEY_IDENTIFICATION_SET {
                Self::parse_identification_set(value, &mut report);
            } else if key_tail == KEY_TIMELINE_TRACK {
                if let Some(rate) = Self::parse_timeline_track_edit_rate(value) {
                    if track_edit_rate == 0.0 {
                        track_edit_rate = rate;
                    }
                }
            } else if key_tail == KEY_SOURCE_CLIP {
                if let Some(dur) = Self::parse_source_clip_duration(value) {
                    if dur > track_duration_edit_units {
                        track_duration_edit_units = dur;
                    }
                }
            }
        }

        // Calculate overall duration from edit rate and duration edit units
        if track_edit_rate > 0.0 && track_duration_edit_units > 0 {
            let dur_ms = (track_duration_edit_units as f64 / track_edit_rate) * 1000.0;
            if dur_ms > 0.0 {
                report.general.duration_ms = Some(dur_ms);
                if has_video {
                    video_track.duration_ms = Some(dur_ms);
                }
                if has_audio {
                    audio_track.duration_ms = Some(dur_ms);
                }
                let br = ((data.len() as u64 * 8) as f64 / (dur_ms / 1000.0)) as u64;
                report.general.overall_bitrate = Some(br);
            }
        }

        if has_video {
            report.videos.push(video_track);
        }
        if has_audio {
            report.audios.push(audio_track);
        }

        Ok(report)
    }

    fn parse_ber_length(data: &[u8]) -> Result<(usize, usize)> {
        if data.is_empty() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }
        let first = data[0];
        if first < 0x80 {
            Ok((first as usize, 1))
        } else {
            let num_bytes = (first & 0x7F) as usize;
            if num_bytes == 0 || num_bytes > 8 || data.len() < 1 + num_bytes {
                return Err(MediaInfoError::InvalidData(
                    "Invalid BER length".to_string(),
                ));
            }
            let mut val = 0usize;
            for &b in &data[1..1 + num_bytes] {
                val = (val << 8) | (b as usize);
            }
            Ok((val, 1 + num_bytes))
        }
    }

    fn parse_picture_essence_descriptor(data: &[u8], track: &mut VideoTrack) {
        let mut offset = 0;
        while offset + 4 <= data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let val = &data[offset..offset + len];
            offset += len;

            match tag {
                0x3203 if val.len() >= 4 => {
                    // StoredWidth
                    track.width = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                }
                0x3202 if val.len() >= 4 => {
                    // StoredHeight
                    track.height = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                }
                0x3001 if val.len() >= 8 => {
                    // SampleRate (Rational)
                    let num = u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as f64;
                    let den = u32::from_be_bytes([val[4], val[5], val[6], val[7]]) as f64;
                    if den > 0.0 {
                        track.frame_rate = Some(num / den);
                    }
                }
                0x320C if !val.is_empty() => {
                    // FrameLayout: 0=FullFrame/Progressive, 1=SeparateFields/Interlaced
                    track.scan_type = Some(if val[0] == 0 {
                        "Progressive".to_string()
                    } else {
                        "Interlaced".to_string()
                    });
                }
                0x3201 if val.len() >= 16 => {
                    // PictureEssenceCoding (16-byte UL)
                    track.format = Self::ul_to_video_codec(val);
                }
                _ => {}
            }
        }
    }

    fn parse_sound_essence_descriptor(data: &[u8], track: &mut AudioTrack) {
        track.format = AudioCodec::PCM;
        track.format_info = Some("Pulse Code Modulation".to_string());
        track.compression_mode = Some("Lossless".to_string());

        let mut offset = 0;
        while offset + 4 <= data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let val = &data[offset..offset + len];
            offset += len;

            match tag {
                0x3D03 if val.len() >= 8 => {
                    // AudioSamplingRate (Rational)
                    let num = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                    let den = u32::from_be_bytes([val[4], val[5], val[6], val[7]]);
                    if den > 0 {
                        track.sampling_rate = num / den;
                    }
                }
                0x3D07 if val.len() >= 4 => {
                    // ChannelCount
                    let ch = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                    track.channels = ch;
                    track.channel_layout = match ch {
                        1 => Some(AudioChannelLayout::Mono),
                        2 => Some(AudioChannelLayout::Stereo),
                        6 => Some(AudioChannelLayout::Surround5_1),
                        8 => Some(AudioChannelLayout::Surround7_1),
                        _ => Some(AudioChannelLayout::Stereo),
                    };
                }
                0x3D01 if val.len() >= 4 => {
                    // QuantizationBits
                    track.bit_depth =
                        Some(u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as u8);
                }
                _ => {}
            }
        }
    }

    fn parse_identification_set(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        while offset + 4 <= data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let val = &data[offset..offset + len];
            offset += len;

            match tag {
                0x3C01 | 0x3C02 if val.len() >= 2 => {
                    // CompanyName or ProductName (UTF-16BE)
                    let u16_slice: Vec<u16> = val
                        .chunks_exact(2)
                        .map(|c| u16::from_be_bytes([c[0], c[1]]))
                        .take_while(|&c| c != 0)
                        .collect();
                    if let Ok(s) = String::from_utf16(&u16_slice) {
                        if !s.is_empty() {
                            report.general.encoded_application = Some(s);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_timeline_track_edit_rate(data: &[u8]) -> Option<f64> {
        let mut offset = 0;
        while offset + 4 <= data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + len > data.len() {
                break;
            }
            let val = &data[offset..offset + len];
            offset += len;

            if tag == 0x4B01 && val.len() >= 8 {
                let num = u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as f64;
                let den = u32::from_be_bytes([val[4], val[5], val[6], val[7]]) as f64;
                if den > 0.0 {
                    return Some(num / den);
                }
            }
        }
        None
    }

    fn parse_source_clip_duration(data: &[u8]) -> Option<u64> {
        let mut offset = 0;
        while offset + 4 <= data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + len > data.len() {
                break;
            }
            let val = &data[offset..offset + len];
            offset += len;

            if tag == 0x0202 && val.len() >= 8 {
                // Duration in edit units
                return Some(u64::from_be_bytes([
                    val[0], val[1], val[2], val[3], val[4], val[5], val[6], val[7],
                ]));
            }
        }
        None
    }

    fn ul_to_video_codec(ul: &[u8]) -> VideoCodec {
        // Match known SMPTE Picture Essence Coding Universal Labels
        if ul.len() < 16 {
            return VideoCodec::Other("MXF Video".to_string());
        }
        // AVC-Intra / H.264
        if ul[0..8] == [0x06, 0x0E, 0x2B, 0x34, 0x04, 0x01, 0x01, 0x0D]
            || ul.windows(4).any(|w| w == b"avc1" || w == b"h264")
        {
            return VideoCodec::AVC;
        }
        // ProRes
        if ul
            .windows(4)
            .any(|w| w == b"apch" || w == b"apcn" || w == b"apcs" || w == b"apco" || w == b"ap4h")
        {
            return VideoCodec::ProRes;
        }
        // DNxHD
        if ul.windows(4).any(|w| w == b"AVdn") {
            return VideoCodec::DNxHD;
        }
        // MPEG-2 Video
        if ul[0..8] == [0x06, 0x0E, 0x2B, 0x34, 0x04, 0x01, 0x01, 0x02] {
            return VideoCodec::MPEG2Video;
        }
        // HEVC
        if ul.windows(4).any(|w| w == b"hvc1" || w == b"hev1") {
            return VideoCodec::HEVC;
        }

        VideoCodec::Other("MXF Video".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mxf_ber_length() {
        assert_eq!(MxfDemuxer::parse_ber_length(&[0x18]).unwrap(), (24, 1));
        assert_eq!(
            MxfDemuxer::parse_ber_length(&[0x82, 0x01, 0x00]).unwrap(),
            (256, 3)
        );
    }
}
