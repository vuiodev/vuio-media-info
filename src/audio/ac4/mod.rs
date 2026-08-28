use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed Dolby AC-4 Audio Header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ac4Header {
    pub syncword: u16,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub bit_depth: u8,
    pub presentations: u8,
}

pub const AC4_SYNC_0: [u8; 2] = [0xAC, 0x40];
pub const AC4_SYNC_1: [u8; 2] = [0xAC, 0x41];

impl Ac4Header {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        let sync = u16::from_be_bytes([data[0], data[1]]);
        if sync != 0xAC40 && sync != 0xAC41 {
            return Err(MediaInfoError::InvalidData(
                "Invalid AC-4 syncword".to_string(),
            ));
        }

        let _bitstream_version = (data[2] >> 5) & 0x07;
        let presentations = (data[2] & 0x1F).max(1);
        let sample_rate = if (data[3] & 0x80) != 0 { 44100 } else { 48000 };

        let channels = 6; // Standard 5.1 presentation default
        let channel_layout = AudioChannelLayout::Surround5_1;

        Ok(Self {
            syncword: sync,
            sample_rate,
            channels,
            channel_layout,
            bit_depth: 24,
            presentations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac4_header() {
        let data = [0xAC, 0x40, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        let ac4 = Ac4Header::parse(&data).unwrap();
        assert_eq!(ac4.syncword, 0xAC40);
        assert_eq!(ac4.sample_rate, 48000);
        assert_eq!(ac4.presentations, 1);
    }
}
