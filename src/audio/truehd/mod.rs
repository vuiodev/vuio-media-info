use crate::core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed Dolby TrueHD / MLP Audio Header.
#[derive(Debug, Clone, PartialEq)]
pub struct TrueHdHeader {
    pub is_truehd: bool,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub bit_depth: u8,
    pub has_atmos: bool,
    pub format_profile: String,
}

pub const TRUEHD_SYNC: [u8; 4] = [0xF8, 0x72, 0x6F, 0xBA];
pub const MLP_SYNC: [u8; 4] = [0xF8, 0x72, 0x6F, 0xA9];

impl TrueHdHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        // Find Major Syncword: 0xF8726FBA (TrueHD) or 0xF8726FA9 (MLP)
        let mut sync_offset = None;
        let mut is_truehd = true;

        for (i, window) in data.windows(4).enumerate().take(4096) {
            if window == TRUEHD_SYNC {
                sync_offset = Some(i);
                is_truehd = true;
                break;
            } else if window == MLP_SYNC {
                sync_offset = Some(i);
                is_truehd = false;
                break;
            }
        }

        let offset = match sync_offset {
            Some(o) => o,
            None => {
                return Err(MediaInfoError::InvalidData(
                    "TrueHD/MLP Major Sync not found".to_string(),
                ));
            }
        };

        if offset + 12 > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: offset + 12,
                actual: data.len(),
            });
        }

        let header_data = &data[offset + 4..];
        let mut r = MsbBitReader::new(header_data);

        let rate_bits = r.read_bits(4)?;
        let sample_rate = match rate_bits {
            0 => 48000,
            1 => 96000,
            2 => 192000,
            8 => 44100,
            9 => 88200,
            10 => 176400,
            _ => 48000,
        };

        let _substreams = r.read_bits(4)?;
        let chan_modifier = r.read_bits(5)?;

        let channels = match chan_modifier {
            1 => 1,
            2 => 2,
            3..=6 => 6,
            7..=12 => 8,
            _ => 6,
        };

        let channel_layout = match channels {
            1 => AudioChannelLayout::Mono,
            2 => AudioChannelLayout::Stereo,
            6 => AudioChannelLayout::Surround5_1,
            8 => AudioChannelLayout::Surround7_1,
            _ => AudioChannelLayout::Surround5_1,
        };

        // Scan subsequent frame data for Dolby Atmos Object Audio Metadata (OAMD sync or substream)
        let has_atmos = is_truehd
            && data
                .windows(4)
                .any(|w| w == [0x72, 0xF8, 0x01, 0x00] || w == [0x00, 0x01, 0xF8, 0x72]);

        let format_profile = if has_atmos {
            "Dolby TrueHD with Dolby Atmos".to_string()
        } else if is_truehd {
            "Dolby TrueHD".to_string()
        } else {
            "Meridian Lossless Packing (MLP)".to_string()
        };

        Ok(Self {
            is_truehd,
            sample_rate,
            channels,
            channel_layout,
            bit_depth: 24,
            has_atmos,
            format_profile,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truehd_header() {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(&TRUEHD_SYNC);
        // rate_bits = 0 (48kHz), substreams = 1, chan_modifier = 6 (5.1)
        data[4] = 1;
        data[5] = 6 << 3;

        let thd = TrueHdHeader::parse(&data).unwrap();
        assert!(thd.is_truehd);
        assert_eq!(thd.sample_rate, 48000);
        assert_eq!(thd.channels, 6);
        assert_eq!(thd.channel_layout, AudioChannelLayout::Surround5_1);
    }
}
