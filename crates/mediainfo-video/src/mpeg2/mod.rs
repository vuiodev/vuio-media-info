use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed MPEG-1 / MPEG-2 Video Sequence Header.
#[derive(Debug, Clone, PartialEq)]
pub struct Mpeg2SequenceHeader {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio_info: u8,
    pub frame_rate: f64,
    pub bit_rate: u64,
    pub chroma_subsampling: ChromaSubsampling,
    pub is_mpeg2: bool,
    pub profile_and_level: Option<String>,
}

impl Mpeg2SequenceHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 12,
                actual: data.len(),
            });
        }

        // Sequence header start code: 0x000001B3
        let mut offset = 0;
        let mut found = false;
        while offset + 4 <= data.len() {
            if data[offset] == 0x00
                && data[offset + 1] == 0x00
                && data[offset + 2] == 0x01
                && data[offset + 3] == 0xB3
            {
                found = true;
                offset += 4;
                break;
            }
            offset += 1;
        }

        if !found || offset + 8 > data.len() {
            return Err(MediaInfoError::InvalidData(
                "MPEG Sequence Header start code (0x000001B3) not found".to_string(),
            ));
        }

        let mut r = MsbBitReader::new(&data[offset..]);

        let width_val = r.read_bits(12)?;
        let height_val = r.read_bits(12)?;
        let aspect_ratio_info = r.read_bits(4)? as u8;
        let frame_rate_code = r.read_bits(4)? as u8;
        let bit_rate_value = r.read_bits(18)?;
        let _marker = r.read_bit()?;
        let _vbv_buffer_size = r.read_bits(10)?;
        let _constrained_param = r.read_bit()?;

        let frame_rate = match frame_rate_code {
            1 => 24000.0 / 1001.0, // 23.976
            2 => 24.0,
            3 => 25.0,
            4 => 30000.0 / 1001.0, // 29.970
            5 => 30.0,
            6 => 50.0,
            7 => 60000.0 / 1001.0, // 59.940
            8 => 60.0,
            _ => 25.0,
        };

        let bit_rate = bit_rate_value as u64 * 400;

        // Check for sequence extension (0x000001B5)
        let is_mpeg2 = data.windows(4).any(|w| w == [0x00, 0x00, 0x01, 0xB5]);

        Ok(Self {
            width: width_val,
            height: height_val,
            aspect_ratio_info,
            frame_rate,
            bit_rate,
            chroma_subsampling: ChromaSubsampling::YUV420,
            is_mpeg2,
            profile_and_level: if is_mpeg2 {
                Some("Main@Main".to_string())
            } else {
                None
            },
        })
    }
}
