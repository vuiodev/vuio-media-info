use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// ASF (Advanced Systems Format - WMA, WMV, VC-1) Container Demuxer.
pub struct AsfDemuxer;

// Top-level GUIDs
const GUID_HEADER: [u8; 16] = [
    0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const GUID_FILE_PROPERTIES: [u8; 16] = [
    0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const GUID_STREAM_PROPERTIES: [u8; 16] = [
    0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const GUID_CONTENT_DESCRIPTION: [u8; 16] = [
    0x33, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const GUID_EXTENDED_CONTENT: [u8; 16] = [
    0x40, 0xA4, 0xD0, 0xD2, 0x07, 0xE3, 0xD2, 0x11, 0x97, 0xF0, 0x00, 0xA0, 0xC9, 0x5E, 0xA8, 0x50,
];

// Stream Type GUIDs
const GUID_STREAM_AUDIO: [u8; 16] = [
    0x40, 0x9E, 0x69, 0xF8, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
];
const GUID_STREAM_VIDEO: [u8; 16] = [
    0xC0, 0xEF, 0x19, 0xBC, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
];

impl AsfDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 30 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 30,
                actual: data.len(),
            });
        }

        if &data[0..16] != GUID_HEADER {
            return Err(MediaInfoError::InvalidData(
                "Not a valid ASF Header Object".to_string(),
            ));
        }

        let header_size = u64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]) as usize;

        let num_sub_objects = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as usize;

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::ASF;
        report.general.file_size = data.len() as u64;

        let end_offset = header_size.min(data.len());
        let mut offset = 30; // after Header Object fixed fields
        let mut parsed_objects = 0;

        while offset + 24 <= end_offset && parsed_objects < num_sub_objects {
            let guid = &data[offset..offset + 16];
            let obj_size = u64::from_le_bytes([
                data[offset + 16],
                data[offset + 17],
                data[offset + 18],
                data[offset + 19],
                data[offset + 20],
                data[offset + 21],
                data[offset + 22],
                data[offset + 23],
            ]) as usize;

            if obj_size < 24 {
                break;
            }

            let payload_offset = offset + 24;
            let payload_len = obj_size.saturating_sub(24);
            let payload_end = (payload_offset + payload_len).min(data.len());

            if payload_offset <= data.len() {
                let payload = &data[payload_offset..payload_end];

                if guid == GUID_FILE_PROPERTIES && payload.len() >= 64 {
                    Self::parse_file_properties(payload, &mut report);
                } else if guid == GUID_STREAM_PROPERTIES && payload.len() >= 54 {
                    Self::parse_stream_properties(payload, &mut report);
                } else if guid == GUID_CONTENT_DESCRIPTION && payload.len() >= 10 {
                    Self::parse_content_description(payload, &mut report);
                } else if guid == GUID_EXTENDED_CONTENT && payload.len() >= 2 {
                    Self::parse_extended_content_description(payload, &mut report);
                }
            }

            offset += obj_size;
            parsed_objects += 1;
        }

        // Set overall bitrate from duration if available
        if let Some(dur_ms) = report.general.duration_ms {
            if dur_ms > 0.0 && report.general.overall_bitrate.is_none() {
                let br = ((data.len() as u64 * 8) as f64 / (dur_ms / 1000.0)) as u64;
                report.general.overall_bitrate = Some(br);
            }
        }

        Ok(report)
    }

    fn parse_file_properties(payload: &[u8], report: &mut MediaReport) {
        // File size (8 bytes), Creation date (8 bytes), Data packets count (8 bytes), Play duration (8 bytes), Send duration (8 bytes), Preroll (8 bytes), Flags (4 bytes), Min packet size (4 bytes), Max packet size (4 bytes), Max bitrate (4 bytes)
        let play_duration_100ns = u64::from_le_bytes([
            payload[40],
            payload[41],
            payload[42],
            payload[43],
            payload[44],
            payload[45],
            payload[46],
            payload[47],
        ]);
        let preroll_ms = u64::from_le_bytes([
            payload[56],
            payload[57],
            payload[58],
            payload[59],
            payload[60],
            payload[61],
            payload[62],
            payload[63],
        ]);

        if play_duration_100ns > 0 {
            let dur_ms = (play_duration_100ns as f64 / 10_000.0) - (preroll_ms as f64);
            let final_dur = if dur_ms > 0.0 {
                dur_ms
            } else {
                play_duration_100ns as f64 / 10_000.0
            };
            report.general.duration_ms = Some(final_dur);
        }

        if payload.len() >= 80 {
            let max_bitrate =
                u32::from_le_bytes([payload[76], payload[77], payload[78], payload[79]]);
            if max_bitrate > 0 {
                report.general.overall_bitrate = Some(max_bitrate as u64);
            }
        }
    }

    fn parse_stream_properties(payload: &[u8], report: &mut MediaReport) {
        let stream_type = &payload[0..16];
        let stream_number = (payload[48] & 0x7F) as u32;

        let type_specific_data_offset = 54; // after stream properties fixed header
        let type_specific_len =
            u32::from_le_bytes([payload[40], payload[41], payload[42], payload[43]]) as usize;

        if payload.len() < type_specific_data_offset + type_specific_len {
            return;
        }

        let specific_data =
            &payload[type_specific_data_offset..type_specific_data_offset + type_specific_len];

        if stream_type == GUID_STREAM_AUDIO && specific_data.len() >= 16 {
            let mut a = AudioTrack::default();
            a.stream_id = stream_number;

            let format_tag = u16::from_le_bytes([specific_data[0], specific_data[1]]);
            let channels = u16::from_le_bytes([specific_data[2], specific_data[3]]) as u32;
            let sample_rate = u32::from_le_bytes([
                specific_data[4],
                specific_data[5],
                specific_data[6],
                specific_data[7],
            ]);
            let byte_rate = u32::from_le_bytes([
                specific_data[8],
                specific_data[9],
                specific_data[10],
                specific_data[11],
            ]);
            let bit_depth = if specific_data.len() >= 16 {
                u16::from_le_bytes([specific_data[14], specific_data[15]]) as u8
            } else {
                16
            };

            a.channels = channels;
            a.sampling_rate = sample_rate;
            a.bit_rate = Some(byte_rate as u64 * 8);
            a.bit_depth = Some(bit_depth);
            a.duration_ms = report.general.duration_ms;

            a.channel_layout = match channels {
                1 => Some(AudioChannelLayout::Mono),
                2 => Some(AudioChannelLayout::Stereo),
                6 => Some(AudioChannelLayout::Surround5_1),
                8 => Some(AudioChannelLayout::Surround7_1),
                _ => Some(AudioChannelLayout::Stereo),
            };

            match format_tag {
                0x0160 => {
                    a.format = AudioCodec::WMA;
                    a.format_info = Some("Windows Media Audio v1".to_string());
                }
                0x0161 => {
                    a.format = AudioCodec::WMA;
                    a.format_info =
                        Some("Windows Media Audio 2 (v7 / v8 / v9 Standard)".to_string());
                }
                0x0162 => {
                    a.format = AudioCodec::WMA;
                    a.format_info = Some("Windows Media Audio 9 Professional".to_string());
                }
                0x0163 => {
                    a.format = AudioCodec::WMA;
                    a.format_info = Some("Windows Media Audio 9 Lossless".to_string());
                    a.compression_mode = Some("Lossless".to_string());
                }
                0x0055 => {
                    a.format = AudioCodec::MPEGAudioLayer3;
                    a.format_info = Some("MPEG Audio Layer 3".to_string());
                }
                0x0001 => {
                    a.format = AudioCodec::PCM;
                    a.format_info = Some("Pulse Code Modulation".to_string());
                }
                _ => {
                    a.format = AudioCodec::Other(format!("WMA (0x{:04X})", format_tag));
                }
            }

            report.audios.push(a);
        } else if stream_type == GUID_STREAM_VIDEO && specific_data.len() >= 40 {
            let mut v = VideoTrack::default();
            v.stream_id = stream_number;

            // BITMAPINFOHEADER starts at offset 11 of specific_data (or 0 if standard)
            let bmp_offset = if specific_data.len() >= 51 { 11 } else { 0 };
            let bmp = &specific_data[bmp_offset..];

            if bmp.len() >= 40 {
                let width = i32::from_le_bytes([bmp[4], bmp[5], bmp[6], bmp[7]]).unsigned_abs();
                let height = i32::from_le_bytes([bmp[8], bmp[9], bmp[10], bmp[11]]).unsigned_abs();
                let bit_depth = u16::from_le_bytes([bmp[14], bmp[15]]) as u8;
                let fourcc = &bmp[16..20];
                let fourcc_str = String::from_utf8_lossy(fourcc).to_string();

                v.width = width;
                v.height = height;
                v.bit_depth = bit_depth;
                v.codec_id = Some(fourcc_str.clone());
                v.duration_ms = report.general.duration_ms;

                match fourcc {
                    b"WMV1" | b"wmv1" => {
                        v.format = VideoCodec::Other("WMV1".to_string());
                        v.format_info = Some("Windows Media Video 7".to_string());
                    }
                    b"WMV2" | b"wmv2" => {
                        v.format = VideoCodec::Other("WMV2".to_string());
                        v.format_info = Some("Windows Media Video 8".to_string());
                    }
                    b"WMV3" | b"wmv3" => {
                        v.format = VideoCodec::VC1;
                        v.format_info =
                            Some("Windows Media Video 9 (VC-1 Simple/Main)".to_string());
                    }
                    b"WVC1" | b"wvc1" | b"WMVA" => {
                        v.format = VideoCodec::VC1;
                        v.format_info = Some("SMPTE 421M (VC-1 Advanced Profile)".to_string());
                    }
                    b"H264" | b"h264" | b"AVC1" | b"avc1" => {
                        v.format = VideoCodec::AVC;
                        v.format_info = Some("Advanced Video Coding (H.264)".to_string());
                    }
                    _ => {
                        v.format = VideoCodec::Other(fourcc_str);
                    }
                }

                report.videos.push(v);
            }
        }
    }

    fn parse_content_description(payload: &[u8], report: &mut MediaReport) {
        if payload.len() < 10 {
            return;
        }
        let title_len = u16::from_le_bytes([payload[0], payload[1]]) as usize;
        let author_len = u16::from_le_bytes([payload[2], payload[3]]) as usize;
        let _copyright_len = u16::from_le_bytes([payload[4], payload[5]]) as usize;
        let _desc_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;
        let _rating_len = u16::from_le_bytes([payload[8], payload[9]]) as usize;

        let mut cur = 10;
        if title_len > 0 && cur + title_len <= payload.len() {
            if let Some(t) = Self::decode_utf16le(&payload[cur..cur + title_len]) {
                report.general.title = Some(t);
            }
            cur += title_len;
        }
        if author_len > 0 && cur + author_len <= payload.len() {
            if let Some(a) = Self::decode_utf16le(&payload[cur..cur + author_len]) {
                report.general.artist = Some(a);
            }
        }
    }

    fn parse_extended_content_description(data: &[u8], report: &mut MediaReport) {
        if data.len() < 2 {
            return;
        }
        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut offset = 2;

        for _ in 0..count {
            if offset + 4 > data.len() {
                break;
            }
            let name_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;
            if offset + name_len > data.len() {
                break;
            }
            let name_bytes = &data[offset..offset + name_len];
            offset += name_len;
            let name_str = Self::decode_utf16le(name_bytes).unwrap_or_default();

            if offset + 4 > data.len() {
                break;
            }
            let val_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let val_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;
            if offset + val_len > data.len() {
                break;
            }
            let val_bytes = &data[offset..offset + val_len];
            offset += val_len;

            if val_type == 0 {
                // Unicode String
                if let Some(val_str) = Self::decode_utf16le(val_bytes) {
                    match name_str.to_ascii_lowercase().as_str() {
                        "wm/albumtitle" | "album" => report.general.album = Some(val_str),
                        "wm/genre" | "genre" => report.general.genre = Some(val_str),
                        "wm/year" | "year" | "date" => report.general.recorded_date = Some(val_str),
                        "wm/albumartist" if report.general.artist.is_none() => {
                            report.general.artist = Some(val_str);
                        }
                        _ => {}
                    }
                }
            } else if val_type == 1 && name_str.eq_ignore_ascii_case("WM/Picture") {
                // Embedded cover picture
                if val_bytes.len() > 10 {
                    report.general.cover_art_present = true;
                    report.general.cover_mime = Some("image/jpeg".to_string());
                }
            }
        }
    }

    fn decode_utf16le(bytes: &[u8]) -> Option<String> {
        let u16_slice: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|&c| c != 0) // strip trailing nulls
            .collect();
        String::from_utf16(&u16_slice).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asf_header_parsing() {
        let mut data = Vec::new();
        // Top-level Header Object
        data.extend_from_slice(&GUID_HEADER);
        data.extend_from_slice(&134u64.to_le_bytes()); // header_size = 30 + 104
        data.extend_from_slice(&1u32.to_le_bytes()); // num_sub_objects = 1
        data.push(0x01); // reserved1
        data.push(0x02); // reserved2

        // Sub-object: File Properties Object (size 80 + 24 = 104 bytes)
        data.extend_from_slice(&GUID_FILE_PROPERTIES);
        data.extend_from_slice(&104u64.to_le_bytes()); // obj_size
        let mut file_props = vec![0u8; 80];
        // play duration: 600,000,000 in 100ns = 60,000 ms = 60s
        let dur_100ns: u64 = 600_000_000;
        file_props[40..48].copy_from_slice(&dur_100ns.to_le_bytes());
        // preroll: 0
        file_props[56..64].copy_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&file_props);

        let report = AsfDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::ASF);
        assert_eq!(report.general.duration_ms, Some(60000.0));
    }
}
