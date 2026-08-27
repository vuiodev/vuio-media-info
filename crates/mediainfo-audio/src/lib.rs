pub mod aac;
pub mod ac3;
pub mod ac4;
pub mod alac;
pub mod amr;
pub mod dts;
pub mod flac;
pub mod mpega;
pub mod mpegh;
pub mod opus;
pub mod pcm;
pub mod truehd;

pub use aac::AacInfo;
pub use ac3::Ac3Header;
pub use ac4::Ac4Header;
pub use alac::AlacSpecificConfig;
pub use amr::AmrInfo;
pub use dts::DtsHeader;
pub use flac::FlacStreamInfo;
pub use mpega::MpegaHeader;
pub use mpegh::MpegHHeader;
pub use opus::OpusHead;
pub use pcm::PcmInfo;
pub use truehd::TrueHdHeader;

#[cfg(test)]
mod tests {
    use super::*;
    use mediainfo_core::types::*;

    #[test]
    fn test_flac_streaminfo() {
        let mut flac_hdr = vec![0u8; 42];
        flac_hdr[0..4].copy_from_slice(b"fLaC");
        flac_hdr[4] = 0x00; // STREAMINFO metadata header
        flac_hdr[5..8].copy_from_slice(&[0, 0, 34]);

        // min block size 4096, max 4096
        flac_hdr[8..10].copy_from_slice(&4096u16.to_be_bytes());
        flac_hdr[10..12].copy_from_slice(&4096u16.to_be_bytes());

        // 44100 Hz, 2 channels, 16 bits, 441000 total samples (10 seconds)
        // 44100 = 0x0AC44 (20 bits), channels=1 (3 bits), bits=15 (5 bits), samples=441000 (36 bits)
        let sample_rate = 44100u32;
        let channels = 1u32; // 2 channels
        let bits_per_sample = 15u32; // 16 bits
        let total_samples = 441000u64;

        let sr_chan_bps = ((sample_rate as u64) << 44)
            | ((channels as u64) << 41)
            | ((bits_per_sample as u64) << 36)
            | (total_samples & 0x0000_000F_FFFF_FFFF);

        flac_hdr[18..26].copy_from_slice(&sr_chan_bps.to_be_bytes());

        let info = FlacStreamInfo::parse(&flac_hdr).unwrap();
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 2);
        assert_eq!(info.bit_depth, 16);
        assert_eq!(info.channel_layout, AudioChannelLayout::Stereo);
        assert!((info.duration_ms - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_mp3_header() {
        // MPEG-1 Layer 3, 320 kbps, 44.1 kHz, Stereo: 0xFF, 0xFB, 0xE0, 0x00
        let mp3_bytes = [0xFF, 0xFB, 0xE0, 0x00];
        let mp3 = MpegaHeader::parse(&mp3_bytes).unwrap();
        assert_eq!(mp3.version, "Version 1");
        assert_eq!(mp3.layer, "Layer 3");
        assert_eq!(mp3.sample_rate, 44100);
        assert_eq!(mp3.bit_rate, 320000);
        assert_eq!(mp3.channels, 2);
    }

    #[test]
    fn test_opus_head() {
        let mut opus_bytes = vec![0u8; 19];
        opus_bytes[0..8].copy_from_slice(b"OpusHead");
        opus_bytes[8] = 1; // version 1
        opus_bytes[9] = 2; // 2 channels
        opus_bytes[10..12].copy_from_slice(&312u16.to_le_bytes()); // pre-skip
        opus_bytes[12..16].copy_from_slice(&48000u32.to_le_bytes()); // 48000

        let opus = OpusHead::parse(&opus_bytes).unwrap();
        assert_eq!(opus.version, 1);
        assert_eq!(opus.channels, 2);
        assert_eq!(opus.channel_layout, AudioChannelLayout::Stereo);
        assert_eq!(opus.original_sample_rate, 48000);
    }
}
