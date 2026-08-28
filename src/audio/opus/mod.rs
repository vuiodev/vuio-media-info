use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed OpusHead header packet.
#[derive(Debug, Clone, PartialEq)]
pub struct OpusHead {
    pub version: u8,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub pre_skip: u16,
    pub original_sample_rate: u32,
    pub output_sample_rate: u32,
}

impl OpusHead {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 19 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 19,
                actual: data.len(),
            });
        }

        if !data.starts_with(b"OpusHead") {
            return Err(MediaInfoError::InvalidData(
                "Not an OpusHead packet (missing 'OpusHead' magic)".to_string(),
            ));
        }

        let version = data[8];
        let channels = data[9] as u32;
        let pre_skip = u16::from_le_bytes([data[10], data[11]]);
        let original_sample_rate = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        let channel_layout = match channels {
            1 => AudioChannelLayout::Mono,
            2 => AudioChannelLayout::Stereo,
            6 => AudioChannelLayout::Surround5_1,
            8 => AudioChannelLayout::Surround7_1,
            _ => AudioChannelLayout::Stereo,
        };

        Ok(Self {
            version,
            channels,
            channel_layout,
            pre_skip,
            original_sample_rate,
            output_sample_rate: 48000,
        })
    }
}
