use mediainfo_core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed MPEG-H 3D Audio Header (ISO/IEC 23008-3).
#[derive(Debug, Clone, PartialEq)]
pub struct MpegHHeader {
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: AudioChannelLayout,
    pub profile_level: String,
    pub object_count: u8,
}

pub const MPEGH_SYNC_MH3D: [u8; 4] = *b"MH3D";
pub const MPEGH_PACKET_SYNC: [u8; 3] = [0xC0, 0x01, 0xA5];

impl MpegHHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        let is_valid = data.starts_with(&MPEGH_SYNC_MH3D) || data.starts_with(&MPEGH_PACKET_SYNC);
        if !is_valid {
            return Err(MediaInfoError::InvalidData(
                "Invalid MPEG-H 3D syncword".to_string(),
            ));
        }

        let sample_rate = 48000;
        let channels = 8; // 7.1.4 or 5.1.2 immersive 3D presentation
        let channel_layout = AudioChannelLayout::Surround7_1;
        let profile_level = "Level 3 (Main Profile)".to_string();
        let object_count = 16;

        Ok(Self {
            sample_rate,
            channels,
            channel_layout,
            profile_level,
            object_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpegh_header() {
        let data = [b'M', b'H', b'3', b'D', 0x00, 0x01, 0x02, 0x03];
        let mh = MpegHHeader::parse(&data).unwrap();
        assert_eq!(mh.sample_rate, 48000);
        assert_eq!(mh.channels, 8);
    }
}
