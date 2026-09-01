#![allow(clippy::collapsible_if, clippy::collapsible_match)]

pub mod av1;
pub mod avc;
pub mod cineform;
pub mod ffv1;
pub mod hevc;
pub mod mpeg2;
pub mod prores;
pub mod vp9;
pub mod vvc;

pub use av1::Av1SequenceHeader;
pub use avc::AvcSps;
pub use cineform::CineFormHeader;
pub use ffv1::Ffv1Header;
pub use hevc::{DolbyVisionRpuParser, HevcSps};
pub use mpeg2::Mpeg2SequenceHeader;
pub use prores::{ProResHeader, ProResPictureHeader, ProResVariant};
pub use vp9::Vp9Header;

/// Video payload carried by one DV DIF sequence, in bytes.
///
/// A DIF sequence is 150 blocks of 80 bytes, of which the video blocks contribute this
/// much actual picture data once the block IDs and macroblock headers are excluded. The
/// figure is fixed by the DV structure, so the video bit rate does not depend on content.
const DV_PAYLOAD_PER_SEQUENCE: u64 = 10_184;

/// Video bit rate of a DV stream, derived from its frame structure.
///
/// `frame_bytes` selects the DV family: DVCPRO50 and DVCPRO HD carry whole multiples of
/// the DV25 frame and scale the payload accordingly.
pub fn dv_video_bitrate(height: u32, frame_rate: f64, frame_bytes: Option<u64>) -> Option<u64> {
    if frame_rate <= 0.0 || height == 0 {
        return None;
    }
    // 625-line systems use 12 DIF sequences per frame, 525-line systems use 10.
    let (sequences, dv25_frame_bytes) = if height >= 550 {
        (12u64, 144_000u64)
    } else {
        (10u64, 120_000u64)
    };

    let multiplier = frame_bytes
        .filter(|bytes| *bytes > 0)
        .map(|bytes| ((bytes as f64 / dv25_frame_bytes as f64).round() as u64).clamp(1, 8))
        .unwrap_or(1);

    let payload_per_frame = DV_PAYLOAD_PER_SEQUENCE * sequences * multiplier;
    Some((payload_per_frame as f64 * 8.0 * frame_rate).round() as u64)
}

/// Reads the header version number from a VC-3 (DNxHD) frame.
///
/// The frame opens with the 0x000002 header prefix followed by the version byte.
pub fn vc3_header_version(data: &[u8]) -> Option<u8> {
    let head = data.get(..5)?;
    if head[0] == 0x00 && head[1] == 0x00 && head[2] == 0x02 && (head[3] & 0xF0) == 0x80 {
        Some(head[4])
    } else {
        None
    }
}

/// Reads the `profile_and_level_indication` from an MPEG-4 Part 2 Visual Object
/// Sequence header (start code 0x000001B0) and names the profile.
pub fn mpeg4_visual_profile(data: &[u8]) -> Option<&'static str> {
    let mut i = 0;
    while i + 4 < data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 && data[i + 3] == 0xB0 {
            return Some(match data[i + 4] {
                0x01..=0x03 => "Simple",
                0x08 => "Simple",
                0x10..=0x12 => "Simple Scalable",
                0x21..=0x25 => "Core",
                0x32..=0x34 => "Main",
                0x42 => "N-bit",
                0xF0..=0xF5 => "Advanced Simple",
                0xB1..=0xB4 => "Advanced Real Time Simple",
                _ => "Simple",
            });
        }
        i += 1;
    }
    None
}
pub use vvc::VvcSps;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::*;

    #[test]
    fn test_hevc_sps_parsing() {
        // Sample HEVC SPS byte vector
        let sps_raw = [
            0x42, 0x01, 0x01, 0x01, 0x60, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03, 0x00, 0x00, 0x03,
            0x00, 0x00, 0x03, 0x00, 0x7B, 0xA0, 0x02, 0x80, 0x80, 0x2D, 0x16, 0x59, 0x99, 0xA4,
            0x93, 0x2B, 0x9A, 0x02, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x00, 0x03, 0x00, 0x79,
            0x1E, 0x3C, 0xD0,
        ];

        let sps = HevcSps::parse(&sps_raw).unwrap();
        assert_eq!(sps.profile_idc, 1);
        assert_eq!(sps.profile_name, "Main");
    }

    #[test]
    fn test_prores_parsing() {
        // Frame header layout per SMPTE RDD 36 section 8.1.
        let mut frame = vec![0u8; 32];
        frame[0..4].copy_from_slice(&32u32.to_be_bytes());
        frame[4..8].copy_from_slice(b"icpf");
        frame[8..10].copy_from_slice(&20u16.to_be_bytes()); // header size
        frame[11] = 0; // bitstream version
        frame[12..16].copy_from_slice(b"apl0"); // encoder identifier
        frame[16..18].copy_from_slice(&1920u16.to_be_bytes());
        frame[18..20].copy_from_slice(&1080u16.to_be_bytes());
        frame[20] = 2 << 6; // chroma 4:2:2, progressive
        frame[21] = 3; // frame rate code 25 fps
        frame[22] = 1; // primaries BT.709
        frame[23] = 1; // transfer BT.709
        frame[24] = 1; // matrix BT.709

        let prores = ProResHeader::parse(&frame).unwrap();
        assert_eq!(prores.width, 1920);
        assert_eq!(prores.height, 1080);
        assert_eq!(prores.chroma_subsampling, ChromaSubsampling::YUV422);
        assert_eq!(prores.scan_type(), "Progressive");
        assert_eq!(prores.frame_rate, Some(25.0));
        assert_eq!(prores.encoder_identifier().as_deref(), Some("apl0"));
        assert!(!prores.alpha_present);
    }

    #[test]
    fn test_prores_variant_from_fourcc() {
        // ap4h is 4444 and ap4x is 4444 XQ; both are 12-bit 4:4:4.
        assert_eq!(ProResVariant::from_fourcc(b"ap4h").profile_name(), "4444");
        assert_eq!(
            ProResVariant::from_fourcc(b"ap4x").profile_name(),
            "4444 XQ"
        );
        assert_eq!(ProResVariant::from_fourcc(b"apch").profile_name(), "422 HQ");
        assert_eq!(
            ProResVariant::from_fourcc(b"apco").profile_name(),
            "422 Proxy"
        );
        assert_eq!(ProResVariant::from_fourcc(b"ap4h").bit_depth(), 12);
        assert_eq!(ProResVariant::from_fourcc(b"apcn").bit_depth(), 10);
        assert_eq!(
            ProResVariant::from_fourcc(b"ap4h").chroma_subsampling(),
            ChromaSubsampling::YUV444
        );
    }
}
