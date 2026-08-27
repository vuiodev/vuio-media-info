use mediainfo_core::types::*;

/// LPCM Audio format description.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmInfo {
    pub bit_depth: u8,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub is_float: bool,
    pub is_big_endian: bool,
}

impl PcmInfo {
    pub fn new(
        bit_depth: u8,
        sample_rate: u32,
        channels: u32,
        is_float: bool,
        is_big_endian: bool,
    ) -> Self {
        let channel_layout = match channels {
            1 => AudioChannelLayout::Mono,
            2 => AudioChannelLayout::Stereo,
            6 => AudioChannelLayout::Surround5_1,
            8 => AudioChannelLayout::Surround7_1,
            _ => AudioChannelLayout::Stereo,
        };

        Self {
            bit_depth,
            sample_rate,
            channels,
            channel_layout,
            is_float,
            is_big_endian,
        }
    }
}
