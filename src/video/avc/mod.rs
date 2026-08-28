use crate::core::{
    bitstream::{MsbBitReader, unescape_nal_unit},
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed Sequence Parameter Set (SPS) for H.264 / AVC.
#[derive(Debug, Clone, PartialEq)]
pub struct AvcSps {
    pub profile_idc: u8,
    pub profile_name: &'static str,
    pub level_idc: u8,
    pub level_name: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: ChromaSubsampling,
    pub color_range: Option<ColorRange>,
    pub color_primaries: Option<ColorPrimaries>,
    pub transfer_characteristics: Option<TransferCharacteristics>,
    pub matrix_coefficients: Option<MatrixCoefficients>,
    pub frame_rate: Option<f64>,
    pub sar: Option<(u16, u16)>,
    pub progressive: bool,
}

impl AvcSps {
    /// Parse an SPS NAL unit (with or without NAL unit header, with emulation prevention bytes).
    pub fn parse(raw_sps: &[u8]) -> Result<Self> {
        if raw_sps.is_empty() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        // Unescape emulation prevention bytes
        let unescaped = unescape_nal_unit(raw_sps);
        let mut slice = unescaped.as_slice();

        // If leading NAL header byte is present (NAL type 7 for SPS), skip it
        if !slice.is_empty() && (slice[0] & 0x1F) == 7 {
            slice = &slice[1..];
        }

        let mut r = MsbBitReader::new(slice);

        let profile_idc = r.read_u8()?;
        let _constraint_flags = r.read_u8()?;
        let level_idc = r.read_u8()?;
        let _sps_id = r.read_ue()?;

        let mut chroma_format_idc = 1u32; // Default 4:2:0
        let mut bit_depth_luma_minus8 = 0u32;
        let mut _bit_depth_chroma_minus8 = 0u32;

        if matches!(
            profile_idc,
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
        ) {
            chroma_format_idc = r.read_ue()?;
            if chroma_format_idc == 3 {
                let _separate_colour_plane_flag = r.read_bit()?;
            }
            bit_depth_luma_minus8 = r.read_ue()?;
            _bit_depth_chroma_minus8 = r.read_ue()?;
            let _qpprime_y_zero = r.read_bit()?;
            let seq_scaling_matrix_present = r.read_bit()?;
            if seq_scaling_matrix_present {
                let count = if chroma_format_idc != 3 { 8 } else { 12 };
                for _ in 0..count {
                    let seq_scaling_list_present = r.read_bit()?;
                    if seq_scaling_list_present {
                        // Skip scaling list
                        let size = 16;
                        let mut last_scale = 8i32;
                        let mut next_scale = 8i32;
                        for _ in 0..size {
                            if next_scale != 0 {
                                let delta_scale = r.read_se()?;
                                next_scale = (last_scale + delta_scale + 256) % 256;
                            }
                            last_scale = if next_scale == 0 {
                                last_scale
                            } else {
                                next_scale
                            };
                        }
                    }
                }
            }
        }

        let _log2_max_frame_num = r.read_ue()?;
        let pic_order_cnt_type = r.read_ue()?;
        if pic_order_cnt_type == 0 {
            let _log2_max_pic_order_cnt_lsb = r.read_ue()?;
        } else if pic_order_cnt_type == 1 {
            let _delta_pic_order_always_zero = r.read_bit()?;
            let _offset_for_non_ref_pic = r.read_se()?;
            let _offset_for_top_to_bottom = r.read_se()?;
            let num_ref_frames_in_cycle = r.read_ue()?;
            for _ in 0..num_ref_frames_in_cycle {
                let _ = r.read_se()?;
            }
        }

        let _max_num_ref_frames = r.read_ue()?;
        let _gaps_in_frame_num_value_allowed = r.read_bit()?;

        let pic_width_in_mbs_minus1 = r.read_ue()?;
        let pic_height_in_map_units_minus1 = r.read_ue()?;
        let frame_mbs_only_flag = r.read_bit()?;

        if !frame_mbs_only_flag {
            let _mb_adaptive_frame_field_flag = r.read_bit()?;
        }

        let _direct_8x8_inference_flag = r.read_bit()?;
        let frame_cropping_flag = r.read_bit()?;

        let mut crop_left = 0u32;
        let mut crop_right = 0u32;
        let mut crop_top = 0u32;
        let mut crop_bottom = 0u32;

        if frame_cropping_flag {
            crop_left = r.read_ue()?;
            crop_right = r.read_ue()?;
            crop_top = r.read_ue()?;
            crop_bottom = r.read_ue()?;
        }

        let crop_unit_x = match chroma_format_idc {
            0 => 1,
            1 | 2 => 2,
            3 => 1,
            _ => 1,
        };
        let crop_unit_y = match chroma_format_idc {
            0 => 2 - if frame_mbs_only_flag { 1 } else { 0 },
            1 => 2 * (2 - if frame_mbs_only_flag { 1 } else { 0 }),
            2 => 2 - if frame_mbs_only_flag { 1 } else { 0 },
            3 => 2 - if frame_mbs_only_flag { 1 } else { 0 },
            _ => 1,
        };

        let raw_width = (pic_width_in_mbs_minus1 + 1) * 16;
        let raw_height = (pic_height_in_map_units_minus1 + 1)
            * 16
            * (2 - if frame_mbs_only_flag { 1 } else { 0 });

        let width = raw_width.saturating_sub((crop_left + crop_right) * crop_unit_x);
        let height = raw_height.saturating_sub((crop_top + crop_bottom) * crop_unit_y);

        // VUI Parameters
        let vui_parameters_present_flag = r.read_bit().unwrap_or(false);
        let mut sar = None;
        let mut color_range = None;
        let mut color_primaries = None;
        let mut transfer_characteristics = None;
        let mut matrix_coefficients = None;
        let mut frame_rate = None;

        if vui_parameters_present_flag {
            let aspect_ratio_info_present = r.read_bit().unwrap_or(false);
            if aspect_ratio_info_present {
                let aspect_ratio_idc = r.read_u8().unwrap_or(0);
                if aspect_ratio_idc == 255 {
                    // Extended_SAR
                    let sar_w = r.read_u16_be().unwrap_or(1);
                    let sar_h = r.read_u16_be().unwrap_or(1);
                    sar = Some((sar_w, sar_h));
                }
            }

            let overscan_info_present = r.read_bit().unwrap_or(false);
            if overscan_info_present {
                let _ = r.read_bit();
            }

            let video_signal_type_present = r.read_bit().unwrap_or(false);
            if video_signal_type_present {
                let _video_format = r.read_bits(3).unwrap_or(5);
                let full_range = r.read_bit().unwrap_or(false);
                color_range = Some(if full_range {
                    ColorRange::Full
                } else {
                    ColorRange::Limited
                });

                let colour_description_present = r.read_bit().unwrap_or(false);
                if colour_description_present {
                    let cp = r.read_u8().unwrap_or(2);
                    let tc = r.read_u8().unwrap_or(2);
                    let mc = r.read_u8().unwrap_or(2);
                    color_primaries = Some(ColorPrimaries::from_u8(cp));
                    transfer_characteristics = Some(TransferCharacteristics::from_u8(tc));
                    matrix_coefficients = Some(MatrixCoefficients::from_u8(mc));
                }
            }

            let chroma_loc_info_present = r.read_bit().unwrap_or(false);
            if chroma_loc_info_present {
                let _ = r.read_ue();
                let _ = r.read_ue();
            }

            let timing_info_present = r.read_bit().unwrap_or(false);
            if timing_info_present {
                if let (Ok(num_units), Ok(time_scale)) = (r.read_u32_be(), r.read_u32_be()) {
                    let _fixed_rate = r.read_bit().unwrap_or(false);
                    if num_units > 0 {
                        frame_rate = Some(time_scale as f64 / (2.0 * num_units as f64));
                    }
                }
            }
        }

        let profile_name = match profile_idc {
            66 => "Baseline",
            77 => "Main",
            88 => "Extended",
            100 => "High",
            110 => "High 10",
            122 => "High 4:2:2",
            244 => "High 4:4:4 Predictive",
            _ => "High",
        };

        let level_name = format!("{}.{}", level_idc / 10, level_idc % 10);

        let chroma_subsampling = match chroma_format_idc {
            0 => ChromaSubsampling::Monochrome,
            1 => ChromaSubsampling::YUV420,
            2 => ChromaSubsampling::YUV422,
            3 => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV420,
        };

        Ok(Self {
            profile_idc,
            profile_name,
            level_idc,
            level_name,
            width,
            height,
            bit_depth: (8 + bit_depth_luma_minus8) as u8,
            chroma_subsampling,
            color_range,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
            frame_rate,
            sar,
            progressive: frame_mbs_only_flag,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_avc_sps_1080p() {
        // High Profile @ Level 4.0 1920x1080 23.976fps BT.709 SPS
        let sps_bytes = [
            0x67, 0x64, 0x00, 0x28, 0xAC, 0xD9, 0x40, 0x78, 0x02, 0x27, 0xE5, 0x84, 0x00, 0x00,
            0x03, 0x00, 0x04, 0x00, 0x00, 0x03, 0x00, 0xF0, 0x3C, 0x60, 0xC6, 0x58,
        ];

        let sps = AvcSps::parse(&sps_bytes).unwrap();
        assert_eq!(sps.profile_idc, 100);
        assert_eq!(sps.profile_name, "High");
        assert_eq!(sps.level_idc, 40);
        assert_eq!(sps.level_name, "4.0");
        assert_eq!(sps.width, 1920);
        assert_eq!(sps.height, 1080);
        assert_eq!(sps.bit_depth, 8);
        assert_eq!(sps.chroma_subsampling, ChromaSubsampling::YUV420);
    }
}
