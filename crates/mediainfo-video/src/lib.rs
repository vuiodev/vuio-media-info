#![allow(clippy::collapsible_if, clippy::collapsible_match)]

pub mod av1;
pub mod avc;
pub mod cineform;
pub mod hevc;
pub mod mpeg2;
pub mod prores;
pub mod vp9;
pub mod vvc;

pub use av1::Av1SequenceHeader;
pub use avc::AvcSps;
pub use cineform::CineFormHeader;
pub use hevc::{DolbyVisionRpuParser, HevcSps};
pub use mpeg2::Mpeg2SequenceHeader;
pub use prores::ProResHeader;
pub use vp9::Vp9Header;
pub use vvc::VvcSps;

#[cfg(test)]
mod tests {
    use super::*;
    use mediainfo_core::types::*;

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
        let mut frame = vec![0u8; 32];
        frame[0..4].copy_from_slice(&32u32.to_be_bytes());
        frame[4..8].copy_from_slice(b"icpf");
        frame[8..10].copy_from_slice(&1920u16.to_be_bytes());
        frame[10..12].copy_from_slice(&1080u16.to_be_bytes());
        frame[12] = 2 << 6; // 4:2:2
        frame[14] = 1; // BT.709
        frame[15] = 1; // BT.709
        frame[16] = 1; // BT.709

        let prores = ProResHeader::parse(&frame).unwrap();
        assert_eq!(prores.width, 1920);
        assert_eq!(prores.height, 1080);
        assert_eq!(prores.chroma_subsampling, ChromaSubsampling::YUV422);
    }
}
