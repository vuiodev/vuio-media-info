use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed Apple Lossless (ALAC) Magic Cookie / Specific Config.
#[derive(Debug, Clone, PartialEq)]
pub struct AlacSpecificConfig {
    pub frame_length: u32,
    pub bit_depth: u8,
    pub channels: u32,
    pub sample_rate: u32,
    pub avg_bitrate: u32,
    pub channel_layout: AudioChannelLayout,
}

impl AlacSpecificConfig {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 24 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 24,
                actual: data.len(),
            });
        }

        let frame_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let _compat_version = data[4];
        let bit_depth = data[5];
        let _pb = data[6];
        let _mb = data[7];
        let _kb = data[8];
        let channels = data[9] as u32;
        let _max_run = u16::from_be_bytes([data[10], data[11]]);
        let _max_frame_bytes = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let avg_bitrate = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let sample_rate = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        let channel_layout = match channels {
            1 => AudioChannelLayout::Mono,
            2 => AudioChannelLayout::Stereo,
            6 => AudioChannelLayout::Surround5_1,
            8 => AudioChannelLayout::Surround7_1,
            _ => AudioChannelLayout::Stereo,
        };

        Ok(Self {
            frame_length,
            bit_depth,
            channels,
            sample_rate,
            avg_bitrate,
            channel_layout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alac_config() {
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&4096u32.to_be_bytes()); // frame length
        data[5] = 24; // 24-bit
        data[9] = 2; // 2 channels
        data[20..24].copy_from_slice(&96000u32.to_be_bytes()); // 96kHz

        let alac = AlacSpecificConfig::parse(&data).unwrap();
        assert_eq!(alac.sample_rate, 96000);
        assert_eq!(alac.bit_depth, 24);
        assert_eq!(alac.channels, 2);
    }
}
