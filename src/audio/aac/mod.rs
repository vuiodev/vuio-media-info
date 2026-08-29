use crate::core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Sampling frequency lookup table for AAC.
pub const AAC_SAMPLING_RATES: [u32; 16] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350, 0, 0,
    0,
];

/// Parsed AAC information from ADTS header or AudioSpecificConfig.
#[derive(Debug, Clone, PartialEq)]
pub struct AacInfo {
    pub profile: &'static str,
    pub sampling_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub is_he_aac: bool,
    pub is_he_aac_v2: bool,
    /// MPEG-4 audio object type, used to build container codec IDs.
    pub audio_object_type: u8,
}

impl AacInfo {
    /// Parse an AAC ADTS frame header (at least 7 bytes).
    pub fn parse_adts(data: &[u8]) -> Result<Self> {
        if data.len() < 7 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 7,
                actual: data.len(),
            });
        }

        let mut r = MsbBitReader::new(data);
        let syncword = r.read_bits(12)?;
        if syncword != 0xFFF {
            return Err(MediaInfoError::InvalidSyncword {
                expected: 0xFFF,
                actual: syncword,
            });
        }

        let _id = r.read_bit()?;
        let _layer = r.read_bits(2)?;
        let _protection_absent = r.read_bit()?;
        let profile_idx = r.read_bits(2)?;
        let sampling_idx = r.read_bits(4)? as usize;
        let _private = r.read_bit()?;
        let channel_config = r.read_bits(3)?;

        let sampling_rate = if sampling_idx < 13 {
            AAC_SAMPLING_RATES[sampling_idx]
        } else {
            44100
        };

        let profile = match profile_idx {
            0 => "Main",
            1 => "LC",
            2 => "SSR",
            3 => "LTP",
            _ => "LC",
        };

        let (channels, channel_layout) = Self::channel_config_to_layout(channel_config as u8);

        Ok(Self {
            profile,
            sampling_rate,
            channels,
            channel_layout,
            is_he_aac: false,
            is_he_aac_v2: false,
            audio_object_type: profile_idx as u8 + 1,
        })
    }

    /// Parse an `AudioSpecificConfig` descriptor (from MP4 `esds` or Matroska CodecPrivate).
    pub fn parse_audio_specific_config(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 2,
                actual: data.len(),
            });
        }

        let mut r = MsbBitReader::new(data);
        let mut audio_object_type = r.read_bits(5)?;
        if audio_object_type == 31 {
            audio_object_type = 32 + r.read_bits(6)?;
        }

        let sampling_idx = r.read_bits(4)? as usize;
        let mut sampling_rate = if sampling_idx == 15 {
            r.read_bits(24)?
        } else if sampling_idx < 13 {
            AAC_SAMPLING_RATES[sampling_idx]
        } else {
            44100
        };

        let channel_config = r.read_bits(4)? as u8;

        let mut is_he_aac = false;
        let mut is_he_aac_v2 = false;

        if audio_object_type == 5 || audio_object_type == 29 {
            is_he_aac = true;
            if audio_object_type == 29 {
                is_he_aac_v2 = true;
            }
            let ext_sampling_idx = r.read_bits(4)? as usize;
            if ext_sampling_idx == 15 {
                sampling_rate = r.read_bits(24)?;
            } else if ext_sampling_idx < 13 {
                sampling_rate = AAC_SAMPLING_RATES[ext_sampling_idx];
            }
        }

        // Check for syncExtensionType (0x2B7 for SBR)
        if r.remaining_bits() >= 16 {
            let sync_ext = r.read_bits(11).unwrap_or(0);
            if sync_ext == 0x2B7 {
                let ext_aot = r.read_bits(5).unwrap_or(0);
                if ext_aot == 5 {
                    is_he_aac = true;
                } else if ext_aot == 29 {
                    is_he_aac_v2 = true;
                }
            }
        }

        let (channels, channel_layout) = Self::channel_config_to_layout(channel_config);

        let profile = if is_he_aac_v2 {
            "HE-AACv2 / LC"
        } else if is_he_aac {
            "HE-AAC / LC"
        } else {
            match audio_object_type {
                1 => "Main",
                2 => "LC",
                3 => "SSR",
                4 => "LTP",
                _ => "LC",
            }
        };

        Ok(Self {
            profile,
            sampling_rate,
            channels,
            channel_layout,
            is_he_aac,
            is_he_aac_v2,
            audio_object_type: audio_object_type as u8,
        })
    }

    fn channel_config_to_layout(config: u8) -> (u32, AudioChannelLayout) {
        match config {
            1 => (1, AudioChannelLayout::Mono),
            2 => (2, AudioChannelLayout::Stereo),
            3 => (3, AudioChannelLayout::Surround3_0),
            4 => (4, AudioChannelLayout::Surround4_0),
            5 => (5, AudioChannelLayout::Surround5_1),
            6 => (6, AudioChannelLayout::Surround5_1),
            7 => (8, AudioChannelLayout::Surround7_1),
            _ => (2, AudioChannelLayout::Stereo),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aac_asc() {
        let data = [0x12, 0x10];
        let info = AacInfo::parse_audio_specific_config(&data).unwrap();
        assert_eq!(info.profile, "LC");
        assert_eq!(info.sampling_rate, 44100);
        assert_eq!(info.channels, 2);
        assert_eq!(info.channel_layout, AudioChannelLayout::Stereo);
    }
}
