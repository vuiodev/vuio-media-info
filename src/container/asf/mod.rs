use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use std::collections::HashMap;

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
/// ASF_Header_Extension_Object
const GUID_HEADER_EXTENSION: [u8; 16] = [
    0xB5, 0x03, 0xBF, 0x5F, 0x2E, 0xA9, 0xCF, 0x11, 0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
/// ASF_Extended_Stream_Properties_Object
const GUID_EXTENDED_STREAM_PROPERTIES: [u8; 16] = [
    0xCB, 0xA5, 0xE6, 0x14, 0x72, 0xC6, 0x32, 0x43, 0x83, 0x99, 0xA9, 0x69, 0x52, 0x06, 0x5B, 0x5A,
];
/// ASF_Stream_Bitrate_Properties_Object
const GUID_STREAM_BITRATE: [u8; 16] = [
    0xCE, 0x75, 0xF8, 0x7B, 0x8D, 0x46, 0xD1, 0x11, 0x8D, 0x82, 0x00, 0x60, 0x97, 0xC9, 0xA2, 0xB2,
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
                } else if guid == GUID_STREAM_BITRATE && payload.len() >= 2 {
                    Self::parse_stream_bitrate(payload, &mut report);
                } else if guid == GUID_HEADER_EXTENSION && payload.len() > 22 {
                    // The extension data area holds further top-level objects.
                    Self::parse_header_extension(&payload[22..], &mut report);
                }
            }

            offset += obj_size;
            parsed_objects += 1;
        }

        // The header is followed by the Data Object, whose packets carry the per-stream
        // payloads. Walking them is the only way to learn a stream's real size and, for
        // video, its frame rate, when the optional Extended Stream Properties is absent.
        Self::parse_data_object(data, header_size, &mut report);

        // Set overall bitrate from duration if available
        if let Some(dur_ms) = report.general.duration_ms {
            if dur_ms > 0.0 && report.general.overall_bitrate.is_none() {
                let br = ((data.len() as u64 * 8) as f64 / (dur_ms / 1000.0)) as u64;
                report.general.overall_bitrate = Some(br);
            }
        }

        Ok(report)
    }

    /// Walks the Data Object packets, tallying payload bytes per stream and recording
    /// the presentation time of each media object so frame rates can be derived.
    fn parse_data_object(data: &[u8], header_size: usize, report: &mut MediaReport) {
        let packet_size: usize = match report.general.extra.get("PacketSize") {
            Some(v) => match v.parse() {
                Ok(n) if n > 0 => n,
                _ => return,
            },
            None => return,
        };

        // Data Object: GUID(16) size(8) FileID(16) TotalDataPackets(8) Reserved(2).
        let Some(header) = data.get(header_size..header_size + 50) else {
            return;
        };
        let total_packets = u64::from_le_bytes([
            header[40], header[41], header[42], header[43], header[44], header[45], header[46],
            header[47],
        ]);
        let base = header_size + 50;

        let mut payload_bytes: HashMap<u8, u64> = HashMap::new();
        // First and last media object presentation time, plus a count, per stream.
        let mut timing: HashMap<u8, (u32, u32, u64)> = HashMap::new();

        for i in 0..total_packets {
            let start = base + (i as usize) * packet_size;
            let Some(packet) = data.get(start..start + packet_size) else {
                break;
            };
            Self::parse_data_packet(packet, &mut payload_bytes, &mut timing);
        }

        let seconds = report.general.duration_ms.unwrap_or(0.0) / 1000.0;
        for v in &mut report.videos {
            let Ok(stream) = u8::try_from(v.stream_id) else {
                continue;
            };
            if let Some(&bytes) = payload_bytes.get(&stream) {
                v.stream_size = Some(bytes);
                if v.bit_rate.is_none() && seconds > 0.0 {
                    v.bit_rate = Some((bytes as f64 * 8.0 / seconds) as u64);
                }
            }
            if v.frame_rate.is_none() {
                if let Some(&(first, last, count)) = timing.get(&stream) {
                    if count > 1 && last > first {
                        let span_s = (last - first) as f64 / 1000.0;
                        v.frame_rate = Some((count - 1) as f64 / span_s);
                        v.frame_rate_mode = Some(FrameRateMode::Constant);
                    }
                }
                v.frame_count = timing.get(&stream).map(|&(_, _, c)| c);
            }
        }
        for a in &mut report.audios {
            let Ok(stream) = u8::try_from(a.stream_id) else {
                continue;
            };
            if let Some(&bytes) = payload_bytes.get(&stream) {
                a.stream_size = Some(bytes);
            }
        }
    }

    /// Reads one Data Object packet, which may carry a single payload or several.
    fn parse_data_packet(
        packet: &[u8],
        payload_bytes: &mut HashMap<u8, u64>,
        timing: &mut HashMap<u8, (u32, u32, u64)>,
    ) {
        // Field widths are selected by 2-bit type codes throughout the packet header.
        fn read_var(data: &[u8], pos: &mut usize, kind: u8) -> Option<u32> {
            let width = match kind {
                0 => return Some(0),
                1 => 1,
                2 => 2,
                _ => 4,
            };
            let bytes = data.get(*pos..*pos + width)?;
            *pos += width;
            Some(
                bytes
                    .iter()
                    .rev()
                    .fold(0u32, |acc, &b| (acc << 8) | b as u32),
            )
        }

        let mut pos = 0usize;
        let Some(&first) = packet.first() else { return };
        if first & 0x80 != 0 {
            // Error correction data present; its length is in the low nibble.
            pos += 1 + (first & 0x0F) as usize;
        }

        let (Some(&length_flags), Some(&property_flags)) = (packet.get(pos), packet.get(pos + 1))
        else {
            return;
        };
        pos += 2;

        let Some(packet_length) = read_var(packet, &mut pos, (length_flags >> 5) & 0x03) else {
            return;
        };
        if read_var(packet, &mut pos, (length_flags >> 1) & 0x03).is_none() {
            return;
        }
        let Some(padding) = read_var(packet, &mut pos, (length_flags >> 3) & 0x03) else {
            return;
        };
        // Send time (4 bytes) and duration (2 bytes).
        pos += 6;

        let multiple = length_flags & 0x01 != 0;
        let (count, payload_length_kind) = if multiple {
            let Some(&flags) = packet.get(pos) else {
                return;
            };
            pos += 1;
            (flags & 0x3F, (flags >> 6) & 0x03)
        } else {
            (1, 0)
        };

        for _ in 0..count {
            let Some(&stream_byte) = packet.get(pos) else {
                return;
            };
            pos += 1;
            let stream = stream_byte & 0x7F;

            if read_var(packet, &mut pos, (property_flags >> 4) & 0x03).is_none() {
                return;
            }
            let Some(offset_into_object) = read_var(packet, &mut pos, (property_flags >> 2) & 0x03)
            else {
                return;
            };
            let Some(replicated_len) = read_var(packet, &mut pos, property_flags & 0x03) else {
                return;
            };
            let replicated_start = pos;
            pos += replicated_len as usize;

            let length = if multiple {
                match read_var(packet, &mut pos, payload_length_kind) {
                    Some(len) => len as usize,
                    None => return,
                }
            } else {
                let total = if packet_length > 0 {
                    packet_length as usize
                } else {
                    packet.len()
                };
                total.saturating_sub(pos).saturating_sub(padding as usize)
            };

            *payload_bytes.entry(stream).or_insert(0) += length as u64;

            // A zero offset starts a new media object; its presentation time sits in the
            // replicated data right after the media object size.
            if offset_into_object == 0 && replicated_len >= 8 {
                if let Some(bytes) = packet.get(replicated_start + 4..replicated_start + 8) {
                    let time = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    timing
                        .entry(stream)
                        .and_modify(|entry| {
                            entry.0 = entry.0.min(time);
                            entry.1 = entry.1.max(time);
                            entry.2 += 1;
                        })
                        .or_insert((time, time, 1));
                }
            }

            pos += length;
        }
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

        if payload.len() >= 72 {
            let min_packet_size =
                u32::from_le_bytes([payload[68], payload[69], payload[70], payload[71]]);
            if min_packet_size > 0 {
                report
                    .general
                    .extra
                    .insert("PacketSize".to_string(), min_packet_size.to_string());
            }
        }

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

    /// Walks the objects nested inside the Header Extension object.
    fn parse_header_extension(data: &[u8], report: &mut MediaReport) {
        let mut offset = 0;
        while offset + 24 <= data.len() {
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
            let payload_end = (offset + obj_size).min(data.len());
            let payload = &data[offset + 24..payload_end];

            if guid == GUID_EXTENDED_STREAM_PROPERTIES && payload.len() >= 64 {
                Self::parse_extended_stream_properties(payload, report);
            }

            offset += obj_size;
        }
    }

    /// Extended Stream Properties: average frame time and the stream data bit rate.
    fn parse_extended_stream_properties(payload: &[u8], report: &mut MediaReport) {
        let stream_number = u16::from_le_bytes([payload[48], payload[49]]) as u32;
        let data_bitrate = u32::from_le_bytes([payload[40], payload[41], payload[42], payload[43]]);
        // Average time per frame, in 100-nanosecond units.
        let avg_time_per_frame = u64::from_le_bytes([
            payload[56],
            payload[57],
            payload[58],
            payload[59],
            payload[60],
            payload[61],
            payload[62],
            payload[63],
        ]);

        for v in &mut report.videos {
            if v.stream_id != stream_number {
                continue;
            }
            if avg_time_per_frame > 0 {
                v.frame_rate = Some(10_000_000.0 / avg_time_per_frame as f64);
            }
            if data_bitrate > 0 {
                v.bit_rate = Some(data_bitrate as u64);
            }
        }
        for a in &mut report.audios {
            if a.stream_id == stream_number && data_bitrate > 0 && a.bit_rate.is_none() {
                a.bit_rate = Some(data_bitrate as u64);
            }
        }
    }

    /// Stream Bitrate Properties: a per-stream average bit rate table.
    fn parse_stream_bitrate(payload: &[u8], report: &mut MediaReport) {
        let count = u16::from_le_bytes([payload[0], payload[1]]) as usize;
        for i in 0..count {
            let off = 2 + i * 6;
            if off + 6 > payload.len() {
                break;
            }
            let stream_number =
                (u16::from_le_bytes([payload[off], payload[off + 1]]) & 0x7F) as u32;
            let bitrate = u32::from_le_bytes([
                payload[off + 2],
                payload[off + 3],
                payload[off + 4],
                payload[off + 5],
            ]) as u64;
            if bitrate == 0 {
                continue;
            }
            for v in &mut report.videos {
                if v.stream_id == stream_number && v.bit_rate.is_none() {
                    v.bit_rate = Some(bitrate);
                }
            }
            for a in &mut report.audios {
                if a.stream_id == stream_number && a.bit_rate.is_none() {
                    a.bit_rate = Some(bitrate);
                }
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
            a.codec_id = Some(format!("{format_tag:X}"));
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
                // biBitCount is the total across components, not per component.
                let bit_count = u16::from_le_bytes([bmp[14], bmp[15]]);
                let bit_depth = match bit_count {
                    24 | 32 => 8,
                    36 | 48 => 12,
                    n if n >= 8 => (n / 3).clamp(8, 16) as u8,
                    _ => 8,
                };
                let fourcc = &bmp[16..20];
                let fourcc_str = String::from_utf8_lossy(fourcc).to_string();

                v.width = width;
                v.height = height;
                v.bit_depth = bit_depth;
                v.codec_id = Some(fourcc_str.clone());
                v.color_space = Some("YUV".to_string());
                if width > 0 && height > 0 {
                    v.display_aspect_ratio = Some(width as f64 / height as f64);
                }
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
