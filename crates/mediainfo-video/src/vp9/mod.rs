use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed VP9 Video Header / Parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Vp9Header {
    pub profile: u8,
    pub profile_name: &'static str,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: ChromaSubsampling,
    pub color_range: ColorRange,
    pub color_space: u8,
}

impl Vp9Header {
    pub fn parse(raw_frame: &[u8]) -> Result<Self> {
        if raw_frame.is_empty() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let mut r = MsbBitReader::new(raw_frame);

        // Frame marker (must be 0b10)
        let frame_marker = r.read_bits(2)?;
        if frame_marker != 2 {
            return Err(MediaInfoError::InvalidData(
                "Invalid VP9 frame marker".to_string(),
            ));
        }

        let profile_low = r.read_bit()?;
        let profile_high = r.read_bit()?;
        let mut profile = if profile_low { 1 } else { 0 };
        if profile_high {
            profile |= 2;
        }

        if profile == 3 {
            let _reserved = r.read_bit()?;
        }

        let show_existing_frame = r.read_bit()?;
        if show_existing_frame {
            let _frame_to_show = r.read_bits(3)?;
            return Ok(Self {
                profile,
                profile_name: match profile {
                    0 => "Profile 0",
                    1 => "Profile 1",
                    2 => "Profile 2",
                    3 => "Profile 3",
                    _ => "Profile 0",
                },
                width: 0,
                height: 0,
                bit_depth: 8,
                chroma_subsampling: ChromaSubsampling::YUV420,
                color_range: ColorRange::Limited,
                color_space: 0,
            });
        }

        let _frame_type = r.read_bit()?;
        let _show_frame = r.read_bit()?;
        let _error_resilient_mode = r.read_bit()?;

        let mut bit_depth = 8u8;
        if profile >= 2 {
            let high_bit_depth = r.read_bit()?;
            bit_depth = if high_bit_depth { 12 } else { 10 };
        }

        let color_space = r.read_bits(3)? as u8;
        let mut chroma_subsampling = ChromaSubsampling::YUV420;

        let color_range = if color_space != 7 {
            let full_range = r.read_bit()?;
            let range = if full_range {
                ColorRange::Full
            } else {
                ColorRange::Limited
            };

            if profile == 1 || profile == 3 {
                let subsampling_x = r.read_bit()?;
                let subsampling_y = r.read_bit()?;
                let _ = r.read_bit(); // reserved
                chroma_subsampling = if !subsampling_x && !subsampling_y {
                    ChromaSubsampling::YUV444
                } else if subsampling_x && !subsampling_y {
                    ChromaSubsampling::YUV422
                } else {
                    ChromaSubsampling::YUV420
                };
            }
            range
        } else {
            chroma_subsampling = ChromaSubsampling::RGB;
            if profile == 1 || profile == 3 {
                let _ = r.read_bit(); // reserved
            }
            ColorRange::Full
        };

        let width_minus_1 = r.read_bits(16)?;
        let height_minus_1 = r.read_bits(16)?;

        let width = width_minus_1 + 1;
        let height = height_minus_1 + 1;

        let profile_name = match profile {
            0 => "Profile 0 (8-bit 4:2:0)",
            1 => "Profile 1 (8-bit 4:2:2/4:4:4)",
            2 => "Profile 2 (10/12-bit 4:2:0)",
            3 => "Profile 3 (10/12-bit 4:2:2/4:4:4)",
            _ => "Profile 0",
        };

        Ok(Self {
            profile,
            profile_name,
            width,
            height,
            bit_depth,
            chroma_subsampling,
            color_range,
            color_space,
        })
    }
}
