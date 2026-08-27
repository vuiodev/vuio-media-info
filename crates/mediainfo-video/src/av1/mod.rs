use mediainfo_core::{
    bitstream::MsbBitReader,
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed AV1 Sequence Header OBU.
#[derive(Debug, Clone, PartialEq)]
pub struct Av1SequenceHeader {
    pub profile: u8,
    pub profile_name: &'static str,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: ChromaSubsampling,
    pub color_range: ColorRange,
    pub color_primaries: ColorPrimaries,
    pub transfer_characteristics: TransferCharacteristics,
    pub matrix_coefficients: MatrixCoefficients,
}

impl Av1SequenceHeader {
    pub fn parse(raw_obu: &[u8]) -> Result<Self> {
        if raw_obu.is_empty() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let mut r = MsbBitReader::new(raw_obu);

        // OBU Header
        let _forbidden = r.read_bit()?;
        let obu_type = r.read_bits(4)?;
        let obu_extension = r.read_bit()?;
        let obu_has_size = r.read_bit()?;
        let _reserved = r.read_bit()?;

        if obu_extension {
            let _temporal_id = r.read_bits(3)?;
            let _spatial_id = r.read_bits(2)?;
            let _reserved_ext = r.read_bits(3)?;
        }

        if obu_has_size {
            // Read leb128 size
            let mut _obu_size = 0u64;
            for i in 0..8 {
                let byte = r.read_u8()?;
                _obu_size |= ((byte & 0x7F) as u64) << (i * 7);
                if (byte & 0x80) == 0 {
                    break;
                }
            }
        }

        if obu_type != 1 {
            return Err(MediaInfoError::InvalidData(format!(
                "Expected OBU_SEQUENCE_HEADER (1), found {obu_type}"
            )));
        }

        let seq_profile = r.read_bits(3)? as u8;
        let _still_picture = r.read_bit()?;
        let reduced_still_picture = r.read_bit()?;

        if !reduced_still_picture {
            let timing_info_present = r.read_bit()?;
            if timing_info_present {
                let _num_units = r.read_u32_be()?;
                let _time_scale = r.read_u32_be()?;
                let equal_picture_interval = r.read_bit()?;
                if equal_picture_interval {
                    let _ = r.read_ue();
                }
                let decoder_model_info_present = r.read_bit()?;
                if decoder_model_info_present {
                    let _ = r.read_bits(5); // buffer_delay_length_minus_1
                    let _ = r.read_u32_be(); // num_units_in_decoding_tick
                    let _ = r.read_bits(10); // buffer_removal_time_length_minus_1 + frame_presentation_time_length_minus_1
                }
            }

            let initial_display_delay_present = r.read_bit()?;
            let operating_points_cnt_minus_1 = r.read_bits(5)?;
            for _ in 0..=operating_points_cnt_minus_1 {
                let _op_idc = r.read_bits(12)?;
                let _seq_level_idx = r.read_bits(5)?;
                if _seq_level_idx > 7 {
                    let _seq_tier = r.read_bit()?;
                }
                if initial_display_delay_present {
                    let iddp = r.read_bit()?;
                    if iddp {
                        let _ = r.read_bits(4);
                    }
                }
            }
        }

        let frame_width_bits_minus_1 = r.read_bits(4)?;
        let frame_height_bits_minus_1 = r.read_bits(4)?;

        let max_frame_width_minus_1 = r.read_bits(frame_width_bits_minus_1 as u8 + 1)?;
        let max_frame_height_minus_1 = r.read_bits(frame_height_bits_minus_1 as u8 + 1)?;

        let width = max_frame_width_minus_1 + 1;
        let height = max_frame_height_minus_1 + 1;

        // Color Config
        let high_bitdepth = r.read_bit()?;
        let bit_depth = if seq_profile == 2 && high_bitdepth {
            let twelve_bit = r.read_bit()?;
            if twelve_bit { 12 } else { 10 }
        } else if high_bitdepth {
            10
        } else {
            8
        };

        let mono_chrome = if seq_profile == 1 {
            false
        } else {
            r.read_bit()?
        };

        let color_description_present = r.read_bit()?;
        let mut color_primaries = ColorPrimaries::BT709;
        let mut transfer_characteristics = TransferCharacteristics::BT709;
        let mut matrix_coefficients = MatrixCoefficients::BT709;

        if color_description_present {
            let cp = r.read_u8()?;
            let tc = r.read_u8()?;
            let mc = r.read_u8()?;
            color_primaries = ColorPrimaries::from_u8(cp);
            transfer_characteristics = TransferCharacteristics::from_u8(tc);
            matrix_coefficients = MatrixCoefficients::from_u8(mc);
        }

        let color_range = if mono_chrome {
            let full_range = r.read_bit()?;
            if full_range { ColorRange::Full } else { ColorRange::Limited }
        } else if color_primaries == ColorPrimaries::BT709
            && transfer_characteristics == TransferCharacteristics::IEC61966_2_1
            && matrix_coefficients == MatrixCoefficients::Identity
        {
            ColorRange::Full
        } else {
            let full_range = r.read_bit()?;
            if full_range { ColorRange::Full } else { ColorRange::Limited }
        };

        let mut subsampling_x = true;
        let mut subsampling_y = true;

        if !mono_chrome {
            if seq_profile == 0 {
                subsampling_x = true;
                subsampling_y = true;
            } else if seq_profile == 1 {
                subsampling_x = false;
                subsampling_y = false;
            } else {
                if bit_depth == 12 {
                    subsampling_x = r.read_bit()?;
                    if subsampling_x {
                        subsampling_y = r.read_bit()?;
                    }
                } else {
                    subsampling_x = true;
                    subsampling_y = false;
                }
            }
        }

        let chroma_subsampling = if mono_chrome {
            ChromaSubsampling::Monochrome
        } else if !subsampling_x && !subsampling_y {
            ChromaSubsampling::YUV444
        } else if subsampling_x && !subsampling_y {
            ChromaSubsampling::YUV422
        } else {
            ChromaSubsampling::YUV420
        };

        let profile_name = match seq_profile {
            0 => "Main",
            1 => "High",
            2 => "Professional",
            _ => "Main",
        };

        Ok(Self {
            profile: seq_profile,
            profile_name,
            width,
            height,
            bit_depth,
            chroma_subsampling,
            color_range,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }
}
