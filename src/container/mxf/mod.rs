use crate::core::{
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
const KEY_AES3_DESCRIPTOR: [u8; 4] = [0x01, 0x01, 0x47, 0x00];

/// Renders the second half of a 16-byte UL, the part MediaInfo shows as a codec ID.
fn hex_ul_tail(ul: &[u8]) -> String {
    ul.get(8..16)
        .map(|b| b.iter().map(|x| format!("{x:02X}")).collect())
        .unwrap_or_default()
}
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
        let mut picture_essence_bytes = 0u64;
        let mut sound_essence_bytes = 0u64;
        let mut first_picture_essence: Vec<u8> = Vec::new();

        // The walk covers the whole file: essence element values are skipped by length
        // rather than read, so this stays a hop between KLV headers, and stopping early
        // would under-count the essence the bit rate is derived from.
        while offset + 16 < data.len() {
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
                // The OperationalPattern UL sits after the fixed partition pack fields.
                if let Some(op) = value.get(64..80).and_then(Self::operational_pattern) {
                    report.general.format_profile = Some(op);
                }
                let status_byte = key[13];
                report.general.extra.insert(
                    "PartitionStatus".to_string(),
                    match status_byte {
                        0x02 => "Closed/Complete",
                        0x03 => "Closed/Header",
                        0x04 => "Open/Complete",
                        _ => "Open/Header",
                    }
                    .to_string(),
                );
            }

            // Essence elements: 06.0E.2B.34.01.02.01.01.0D.01.03.01 then the item type.
            if key[4..12] == [0x01, 0x02, 0x01, 0x01, 0x0D, 0x01, 0x03, 0x01] {
                match key[12] {
                    0x05 | 0x15 => {
                        picture_essence_bytes += val_len as u64;
                        if first_picture_essence.is_empty() {
                            first_picture_essence = value[..value.len().min(64)].to_vec();
                        }
                    }
                    0x06 | 0x16 => sound_essence_bytes += val_len as u64,
                    _ => {}
                }
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
                || key_tail == KEY_AES3_DESCRIPTOR
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

        if video_track.format == VideoCodec::DNxHD && video_track.format_version.is_none() {
            video_track.format_version =
                crate::video::vc3_header_version(&first_picture_essence).map(|v| v.to_string());
        }

        // Codecs whose descriptor carries no bit rate get one measured from the essence.
        if let Some(dur_ms) = report.general.duration_ms.filter(|d| *d > 0.0) {
            let seconds = dur_ms / 1000.0;
            if has_video && picture_essence_bytes > 0 {
                video_track.stream_size = Some(picture_essence_bytes);
                if video_track.bit_rate.is_none() {
                    video_track.bit_rate =
                        Some((picture_essence_bytes as f64 * 8.0 / seconds) as u64);
                }
            }
            if has_audio && sound_essence_bytes > 0 {
                audio_track.stream_size = Some(sound_essence_bytes);
                if audio_track.bit_rate.is_none() {
                    audio_track.bit_rate =
                        Some((sound_essence_bytes as f64 * 8.0 / seconds) as u64);
                }
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

    /// Decodes an OperationalPattern UL into its `OP-1a` style name.
    fn operational_pattern(ul: &[u8]) -> Option<String> {
        if ul.len() < 16 || ul[8] != 0x0D || ul[10] != 0x02 {
            return None;
        }
        let item_complexity = ul[12];
        let package_complexity = ul[13];
        // 0x10 marks the specialised OP-Atom pattern rather than a generalised OP.
        if item_complexity == 0x10 {
            return Some("OP-Atom".to_string());
        }
        if (1..=3).contains(&item_complexity) && (1..=3).contains(&package_complexity) {
            let letter = (b'a' + package_complexity - 1) as char;
            return Some(format!("OP-{item_complexity}{letter}"));
        }
        None
    }

    fn parse_picture_essence_descriptor(data: &[u8], track: &mut VideoTrack) {
        let mut offset = 0;
        let mut horizontal_subsampling = None;
        let mut vertical_subsampling = None;
        let mut essence_container: Option<String> = None;
        let mut picture_coding: Option<String> = None;
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
                    let (codec, info, profile) = Self::ul_to_video_codec(val);
                    track.format = codec;
                    track.format_info = info.map(str::to_string);
                    if track.format_profile.is_none() {
                        track.format_profile = profile.map(str::to_string);
                    }
                    picture_coding = Some(hex_ul_tail(val));
                }
                0x3301 if val.len() >= 4 => {
                    // ComponentDepth
                    let depth = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                    if (8..=16).contains(&depth) {
                        track.bit_depth = depth as u8;
                    }
                }
                0x3302 if val.len() >= 4 => {
                    horizontal_subsampling =
                        Some(u32::from_be_bytes([val[0], val[1], val[2], val[3]]));
                }
                0x3308 if val.len() >= 4 => {
                    vertical_subsampling =
                        Some(u32::from_be_bytes([val[0], val[1], val[2], val[3]]));
                }
                0x320E if val.len() >= 8 => {
                    // AspectRatio (Rational)
                    let num = u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as f64;
                    let den = u32::from_be_bytes([val[4], val[5], val[6], val[7]]) as f64;
                    if den > 0.0 {
                        track.display_aspect_ratio = Some(num / den);
                    }
                }
                0x3004 if val.len() >= 16 => {
                    // EssenceContainer UL: the first half of MediaInfo's codec ID.
                    essence_container = Some(hex_ul_tail(val));
                }
                0x3212 if !val.is_empty() => {
                    // FieldDominance: 1 means the first field is field 1 (top).
                    track.scan_order = Some(if val[0] == 2 { "BFF" } else { "TFF" }.to_string());
                }
                0x320D if val.len() >= 16 => {
                    // VideoLineMap: the lower first line marks the dominant field.
                    let f1 = u32::from_be_bytes([val[8], val[9], val[10], val[11]]);
                    let f2 = u32::from_be_bytes([val[12], val[13], val[14], val[15]]);
                    if f1 > 0 && f2 > 0 && track.scan_order.is_none() {
                        track.scan_order = Some(if f1 < f2 { "TFF" } else { "BFF" }.to_string());
                    }
                }
                // MPEGVideoDescriptor BitRate.
                0x8000 if val.len() >= 4 => {
                    let rate = u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as u64;
                    if rate > 0 {
                        track.bit_rate = Some(rate);
                    }
                }
                _ => {}
            }
        }

        // MediaInfo renders the codec ID as the essence container UL joined to the
        // picture essence coding UL.
        track.codec_id = match (&essence_container, &picture_coding) {
            (Some(container), Some(coding)) => Some(format!("{container}-{coding}")),
            (Some(container), None) => Some(container.clone()),
            (None, Some(coding)) => Some(coding.clone()),
            (None, None) => None,
        };

        // SeparateFields stores one field, so the frame height is twice the stored height.
        if track.scan_type.as_deref() == Some("Interlaced") {
            track.stored_height = Some(track.height);
            track.height *= 2;
        }

        track.chroma_subsampling = match (horizontal_subsampling, vertical_subsampling) {
            (Some(1), Some(1)) => Some(ChromaSubsampling::YUV444),
            (Some(2), Some(1)) => Some(ChromaSubsampling::YUV422),
            (Some(2), Some(2)) => Some(ChromaSubsampling::YUV420),
            (Some(4), _) => Some(ChromaSubsampling::YUV411),
            _ => track.chroma_subsampling,
        };
        if track.color_space.is_none() {
            track.color_space = Some("YUV".to_string());
        }
    }

    fn parse_sound_essence_descriptor(data: &[u8], track: &mut AudioTrack) {
        let mut quantization_bits: Option<u8> = None;
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
                0x3004 if val.len() >= 16 => {
                    track.codec_id = Some(hex_ul_tail(val));
                }
                0x3D01 if val.len() >= 4 => {
                    // QuantizationBits
                    let bits = u32::from_be_bytes([val[0], val[1], val[2], val[3]]);
                    if (1..=64).contains(&bits) {
                        quantization_bits = Some(bits as u8);
                    }
                }
                0x3D09 if val.len() >= 4 => {
                    // AvgBps, in bytes per second.
                    let bps = u32::from_be_bytes([val[0], val[1], val[2], val[3]]) as u64;
                    if bps > 0 {
                        track.bit_rate = Some(bps * 8);
                        track.bit_rate_mode = Some(BitrateMode::Constant);
                    }
                }
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

        if let Some(bits) = quantization_bits {
            track.bit_depth = Some(bits);
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
                        .as_chunks::<2>()
                        .0
                        .iter()
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

    /// Maps a SMPTE PictureEssenceCoding UL to a codec.
    ///
    /// Byte 12 of the UL selects the compression family and byte 13 the specific scheme,
    /// which is what distinguishes MPEG-2 from AVC, DV from VC-3, and so on.
    fn ul_to_video_codec(ul: &[u8]) -> (VideoCodec, Option<&'static str>, Option<&'static str>) {
        if ul.len() < 16 {
            return (VideoCodec::Other("MXF Video".to_string()), None, None);
        }

        // Uncompressed picture coding lives under a different sub-branch.
        if ul[11] == 0x01 {
            return (VideoCodec::Raw, Some("Uncompressed"), None);
        }

        match (ul[12], ul[13]) {
            (0x01, 0x32) => (VideoCodec::AVC, Some("Advanced Video Coding"), None),
            (0x01, 0x33) => (VideoCodec::HEVC, Some("High Efficiency Video Coding"), None),
            (0x01, 0x20) => (VideoCodec::MPEG4Visual, Some("MPEG-4 Visual"), None),
            (0x01, 0x10) => (VideoCodec::MPEG1Video, Some("MPEG-1 Video"), None),
            (0x01, _) => (
                VideoCodec::MPEG2Video,
                Some("MPEG-2 Video"),
                Some(match ul[14] {
                    0x11 | 0x01 => "Main",
                    0x02 | 0x03 => "High",
                    _ => "Main",
                }),
            ),
            (0x02, _) => (VideoCodec::DV, Some("Digital Video"), None),
            (0x03, 0x06) => (VideoCodec::ProRes, Some("Apple ProRes"), None),
            (0x03, _) => (
                VideoCodec::Other("JPEG".to_string()),
                Some("Motion JPEG"),
                None,
            ),
            (0x0C, _) => (
                VideoCodec::Other("JPEG 2000".to_string()),
                Some("JPEG 2000"),
                None,
            ),
            (0x71, _) => (VideoCodec::DNxHD, Some("Avid DNxHD / VC-3"), Some("HD")),
            _ => (VideoCodec::Other("MXF Video".to_string()), None, None),
        }
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
