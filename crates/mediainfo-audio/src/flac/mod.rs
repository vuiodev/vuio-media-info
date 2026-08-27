use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed FLAC STREAMINFO metadata block.
#[derive(Debug, Clone, PartialEq)]
pub struct FlacStreamInfo {
    pub min_block_size: u16,
    pub max_block_size: u16,
    pub min_frame_size: u32,
    pub max_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub bit_depth: u8,
    pub total_samples: u64,
    pub duration_ms: f64,
}

impl FlacStreamInfo {
    pub fn parse(data: &[u8]) -> Result<Self> {
        // Find 'fLaC' marker or read STREAMINFO directly (34 bytes minimum)
        let mut slice = data;
        if slice.starts_with(b"fLaC") {
            slice = &slice[4..];
        }

        // If metadata block header (4 bytes) is present:
        if slice.len() >= 4 && (slice[0] & 0x7F) == 0 {
            slice = &slice[4..];
        }

        if slice.len() < 34 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 34,
                actual: slice.len(),
            });
        }

        let mut r = MsbBitReader::new(slice);

        let min_block_size = r.read_u16_be()?;
        let max_block_size = r.read_u16_be()?;
        let min_frame_size = r.read_u24_be()?;
        let max_frame_size = r.read_u24_be()?;

        let sample_rate = r.read_bits(20)?;
        let channels_minus1 = r.read_bits(3)?;
        let channels = channels_minus1 + 1;
        let bits_per_sample_minus1 = r.read_bits(5)? as u8;
        let bit_depth = bits_per_sample_minus1 + 1;
        let total_samples = r.read_bits_u64(36)?;

        let duration_ms = if sample_rate > 0 {
            (total_samples as f64 / sample_rate as f64) * 1000.0
        } else {
            0.0
        };

        let channel_layout = match channels {
            1 => AudioChannelLayout::Mono,
            2 => AudioChannelLayout::Stereo,
            6 => AudioChannelLayout::Surround5_1,
            8 => AudioChannelLayout::Surround7_1,
            _ => AudioChannelLayout::Stereo,
        };

        Ok(Self {
            min_block_size,
            max_block_size,
            min_frame_size,
            max_frame_size,
            sample_rate,
            channels,
            channel_layout,
            bit_depth,
            total_samples,
            duration_ms,
        })
    }
}
