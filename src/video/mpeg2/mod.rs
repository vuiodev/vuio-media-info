use crate::core::{
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

        let mut width = width_val;
        let mut height = height_val;
        let mut chroma_subsampling = ChromaSubsampling::YUV420;
        let mut is_mpeg2 = false;
        let mut _is_progressive = false;
        let mut profile_and_level = None;

        // Search for Sequence Extension (0x000001B5)
        if let Some(ext_idx) = data.windows(4).position(|w| w == [0x00, 0x00, 0x01, 0xB5]) {
            if ext_idx + 10 <= data.len() {
                let ext_data = &data[ext_idx + 4..];
                if let Ok(mut er) = MsbBitReader::new(ext_data)
                    .read_bits(4)
                    .map(|id| (id, MsbBitReader::new(&ext_data[1..])))
                {
                    if er.0 == 1 {
                        // Sequence Extension ID = 1
                        is_mpeg2 = true;
                        if let Ok(profile_and_level_code) = er.1.read_bits(8) {
                            let profile = match (profile_and_level_code >> 4) & 0x07 {
                                1 => "High",
                                2 => "Spatially Scalable",
                                3 => "SNR Scalable",
                                4 => "Main",
                                5 => "Simple",
                                _ => "Main",
                            };
                            let level = match profile_and_level_code & 0x0F {
                                4 => "High",
                                6 => "High 1440",
                                8 => "Main",
                                10 => "Low",
                                _ => "Main",
                            };
                            profile_and_level = Some(format!("{}@{}", profile, level));
                        }
                        if let Ok(prog) = er.1.read_bit() {
                            _is_progressive = prog;
                        }
                        if let Ok(chroma_format) = er.1.read_bits(2) {
                            chroma_subsampling = match chroma_format {
                                1 => ChromaSubsampling::YUV420,
                                2 => ChromaSubsampling::YUV422,
                                3 => ChromaSubsampling::YUV444,
                                _ => ChromaSubsampling::YUV420,
                            };
                        }
                        if let (Ok(h_ext), Ok(v_ext)) = (er.1.read_bits(2), er.1.read_bits(2)) {
                            width |= h_ext << 12;
                            height |= v_ext << 12;
                        }
                    }
                }
            }
        }

        Ok(Self {
            width,
            height,
            aspect_ratio_info,
            frame_rate,
            bit_rate,
            chroma_subsampling,
            is_mpeg2,
            profile_and_level: if is_mpeg2 {
                profile_and_level.or(Some("Main@Main".to_string()))
            } else {
                None
            },
        })
    }
}
