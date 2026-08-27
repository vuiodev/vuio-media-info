use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

pub const MP3_BITRATES_V1_L3: [u32; 16] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
];

pub const MP3_SAMPLE_RATES_V1: [u32; 4] = [44100, 48000, 32000, 0];
pub const MP3_SAMPLE_RATES_V2: [u32; 4] = [22050, 24000, 16000, 0];
pub const MP3_SAMPLE_RATES_V2_5: [u32; 4] = [11025, 12000, 8000, 0];

/// Parsed MPEG Audio frame header.
#[derive(Debug, Clone, PartialEq)]
pub struct MpegaHeader {
    pub version: &'static str,
    pub layer: &'static str,
    pub sample_rate: u32,
    pub bit_rate: u64,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub is_vbr: bool,
    pub frame_size: usize,
}

impl MpegaHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 4,
                actual: data.len(),
            });
        }

        let mut r = MsbBitReader::new(data);
        let syncword = r.read_bits(11)?;
        if syncword != 0x7FF {
            return Err(MediaInfoError::InvalidSyncword {
                expected: 0x7FF,
                actual: syncword,
            });
        }

        let version_id = r.read_bits(2)?; // 0: MPEG 2.5, 2: MPEG 2, 3: MPEG 1
        let layer_id = r.read_bits(2)?; // 1: Layer III, 2: Layer II, 3: Layer I
        let _protection_bit = r.read_bit()?;
        let bitrate_idx = r.read_bits(4)? as usize;
        let sampling_idx = r.read_bits(2)? as usize;
        let padding_bit = r.read_bit()?;
        let _private_bit = r.read_bit()?;
        let channel_mode = r.read_bits(2)?;

        let version = match version_id {
            0 => "Version 2.5",
            2 => "Version 2",
            3 => "Version 1",
            _ => "Version 1",
        };

        let layer = match layer_id {
            1 => "Layer 3",
            2 => "Layer 2",
            3 => "Layer 1",
            _ => "Layer 3",
        };

        let sample_rate = match version_id {
            0 => MP3_SAMPLE_RATES_V2_5[sampling_idx],
            2 => MP3_SAMPLE_RATES_V2[sampling_idx],
            _ => MP3_SAMPLE_RATES_V1[sampling_idx],
        };

        let bit_rate = if bitrate_idx < MP3_BITRATES_V1_L3.len() {
            MP3_BITRATES_V1_L3[bitrate_idx] as u64 * 1000
        } else {
            128000
        };

        let (channels, channel_layout) = match channel_mode {
            3 => (1, AudioChannelLayout::Mono),
            _ => (2, AudioChannelLayout::Stereo),
        };

        let frame_size = if sample_rate > 0 && bit_rate > 0 {
            let padding = if padding_bit { 1 } else { 0 };
            ((144 * bit_rate / sample_rate as u64) + padding) as usize
        } else {
            418
        };

        // Check for Xing / Info / VBRI VBR header inside the frame
        let is_vbr = data.windows(4).any(|w| w == b"Xing" || w == b"Info" || w == b"VBRI");

        Ok(Self {
            version,
            layer,
            sample_rate,
            bit_rate,
            channels,
            channel_layout,
            is_vbr,
            frame_size,
        })
    }
}
