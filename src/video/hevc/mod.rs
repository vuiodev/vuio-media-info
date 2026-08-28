use crate::core::{
    bitstream::{MsbBitReader, unescape_nal_unit},
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed Sequence Parameter Set (SPS) for H.265 / HEVC.
#[derive(Debug, Clone, PartialEq)]
pub struct HevcSps {
    pub profile_idc: u8,
    pub profile_name: &'static str,
    pub tier: &'static str,
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
    pub hdr_format: Option<String>,
}

impl HevcSps {
    /// Parse an HEVC SPS NAL unit.
    pub fn parse(raw_sps: &[u8]) -> Result<Self> {
        if raw_sps.is_empty() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 1,
                actual: 0,
            });
        }

        let unescaped = unescape_nal_unit(raw_sps);
        let mut slice = unescaped.as_slice();

        // If leading 2-byte NAL header is present (HEVC SPS NAL type is 33)
        if slice.len() >= 2 {
            let nal_type = (slice[0] >> 1) & 0x3F;
            if nal_type == 33 {
                slice = &slice[2..];
            }
        }

        let mut r = MsbBitReader::new(slice);

        let _vps_id = r.read_bits(4)?;
        let max_sub_layers_minus1 = r.read_bits(3)?;
        let _temporal_id_nesting = r.read_bit()?;

        // Profile Tier Level
        let _profile_space = r.read_bits(2)?;
        let tier_flag = r.read_bit()?;
        let profile_idc = r.read_bits(5)? as u8;
        let _profile_compat_flags = r.read_u32_be()?;
        let _progressive_source = r.read_bit()?;
        let _interlaced_source = r.read_bit()?;
        let _non_packed_constraint = r.read_bit()?;
        let _frame_only_constraint = r.read_bit()?;
        let _reserved_flags = r.read_bits_u64(44)?;
        let level_idc = r.read_u8()?;

        let mut sub_layer_profile_present = [false; 8];
        let mut sub_layer_level_present = [false; 8];
        for i in 0..max_sub_layers_minus1 {
            sub_layer_profile_present[i as usize] = r.read_bit()?;
            sub_layer_level_present[i as usize] = r.read_bit()?;
        }
        if max_sub_layers_minus1 > 0 {
            for _ in max_sub_layers_minus1..8 {
                let _ = r.read_bits(2);
            }
        }
        for i in 0..max_sub_layers_minus1 as usize {
            if sub_layer_profile_present[i] {
                let _ = r.read_bits_u64(64);
                let _ = r.read_bits_u64(24);
            }
            if sub_layer_level_present[i] {
                let _ = r.read_u8();
            }
        }

        let _sps_id = r.read_ue()?;
        let chroma_format_idc = r.read_ue()?;
        if chroma_format_idc == 3 {
            let _separate_colour_plane = r.read_bit()?;
        }

        let pic_width = r.read_ue()?;
        let pic_height = r.read_ue()?;

        let conformance_window_flag = r.read_bit()?;
        let mut crop_left = 0u32;
        let mut crop_right = 0u32;
        let mut crop_top = 0u32;
        let mut crop_bottom = 0u32;

        if conformance_window_flag {
            crop_left = r.read_ue()?;
            crop_right = r.read_ue()?;
            crop_top = r.read_ue()?;
            crop_bottom = r.read_ue()?;
        }

        let sub_width_c = match chroma_format_idc {
            1 | 2 => 2,
            _ => 1,
        };
        let sub_height_c = match chroma_format_idc {
            1 => 2,
            _ => 1,
        };

        let width = pic_width.saturating_sub((crop_left + crop_right) * sub_width_c);
        let height = pic_height.saturating_sub((crop_top + crop_bottom) * sub_height_c);

        let bit_depth_luma_minus8 = r.read_ue()?;
        let _bit_depth_chroma_minus8 = r.read_ue()?;
        let bit_depth = (8 + bit_depth_luma_minus8) as u8;

        let _log2_max_poc = r.read_ue()?;
        let sps_sub_layer_ordering_info_present = r.read_bit()?;
        let start = if sps_sub_layer_ordering_info_present {
            0
        } else {
            max_sub_layers_minus1
        };
        for _ in start..=max_sub_layers_minus1 {
            let _ = r.read_ue()?;
            let _ = r.read_ue()?;
            let _ = r.read_ue()?;
        }

        let _log2_min_cb = r.read_ue()?;
        let _log2_diff_max_min_cb = r.read_ue()?;
        let _log2_min_tb = r.read_ue()?;
        let _log2_diff_max_min_tb = r.read_ue()?;
        let _max_transform_hierarchy_depth_inter = r.read_ue()?;
        let _max_transform_hierarchy_depth_intra = r.read_ue()?;

        let scaling_list_enabled = r.read_bit()?;
        if scaling_list_enabled {
            let sps_scaling_list_data_present = r.read_bit()?;
            if sps_scaling_list_data_present {
                for size_id in 0..4 {
                    let matrix_count = if size_id == 3 { 2 } else { 6 };
                    for _ in 0..matrix_count {
                        let scaling_list_pred_mode_flag = r.read_bit()?;
                        if !scaling_list_pred_mode_flag {
                            let _ = r.read_ue()?;
                        } else {
                            let coef_num = (1 << (4 + (size_id << 1))).min(64);
                            if size_id > 1 {
                                let _ = r.read_se()?;
                            }
                            for _ in 0..coef_num {
                                let _ = r.read_se()?;
                            }
                        }
                    }
                }
            }
        }

        let _amp_enabled = r.read_bit()?;
        let _sao_enabled = r.read_bit()?;
        let pcm_enabled = r.read_bit()?;
        if pcm_enabled {
            let _ = r.read_bits(4)?;
            let _ = r.read_bits(4)?;
            let _ = r.read_ue()?;
            let _ = r.read_ue()?;
            let _ = r.read_bit()?;
        }

        let num_short_term_ref_pic_sets = r.read_ue()?;
        for _ in 0..num_short_term_ref_pic_sets {
            let inter_ref_pic_set_prediction_flag = r.read_bit().unwrap_or(false);
            if inter_ref_pic_set_prediction_flag {
                let _ = r.read_ue();
                let _ = r.read_bit();
                let _ = r.read_ue();
            } else {
                let num_negative_pics = r.read_ue().unwrap_or(0);
                let num_positive_pics = r.read_ue().unwrap_or(0);
                for _ in 0..num_negative_pics {
                    let _ = r.read_ue();
                    let _ = r.read_bit();
                }
                for _ in 0..num_positive_pics {
                    let _ = r.read_ue();
                    let _ = r.read_bit();
                }
            }
        }

        let long_term_ref_pics_present = r.read_bit().unwrap_or(false);
        if long_term_ref_pics_present {
            let num_long_term_ref_pics = r.read_ue().unwrap_or(0);
            for _ in 0..num_long_term_ref_pics {
                let _ = r.read_bits(4);
                let _ = r.read_bit();
            }
        }

        let _sps_temporal_mvp_enabled = r.read_bit().unwrap_or(false);
        let _strong_intra_smoothing = r.read_bit().unwrap_or(false);

        // VUI Parameters
        let vui_parameters_present = r.read_bit().unwrap_or(false);
        let mut sar = None;
        let mut color_range = None;
        let mut color_primaries = None;
        let mut transfer_characteristics = None;
        let mut matrix_coefficients = None;
        let mut frame_rate = None;

        if vui_parameters_present {
            let aspect_ratio_info_present = r.read_bit().unwrap_or(false);
            if aspect_ratio_info_present {
                let aspect_ratio_idc = r.read_u8().unwrap_or(0);
                if aspect_ratio_idc == 255 {
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

            let _neutral_chroma = r.read_bit().unwrap_or(false);
            let _field_seq = r.read_bit().unwrap_or(false);
            let _frame_field_info = r.read_bit().unwrap_or(false);
            let _default_display_window = r.read_bit().unwrap_or(false);

            let vui_timing_info_present = r.read_bit().unwrap_or(false);
            if vui_timing_info_present {
                if let (Ok(num_units), Ok(time_scale)) = (r.read_u32_be(), r.read_u32_be()) {
                    if num_units > 0 {
                        frame_rate = Some(time_scale as f64 / num_units as f64);
                    }
                }
            }
        }

        let profile_name = match profile_idc {
            1 => "Main",
            2 => "Main 10",
            3 => "Main Still Picture",
            4 => "Format Range Extension",
            _ => "Main",
        };

        let tier = if tier_flag { "High" } else { "Main" };
        let level_name = format!("{}.{}", level_idc / 30, (level_idc % 30) / 3);

        let chroma_subsampling = match chroma_format_idc {
            0 => ChromaSubsampling::Monochrome,
            1 => ChromaSubsampling::YUV420,
            2 => ChromaSubsampling::YUV422,
            3 => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV420,
        };

        let mut hdr_format = None;
        if transfer_characteristics == Some(TransferCharacteristics::SMPTE2084) {
            hdr_format = Some("HDR10".to_string());
        } else if transfer_characteristics == Some(TransferCharacteristics::ARIB_STD_B67) {
            hdr_format = Some("HLG".to_string());
        }

        Ok(Self {
            profile_idc,
            profile_name,
            tier,
            level_idc,
            level_name,
            width,
            height,
            bit_depth,
            chroma_subsampling,
            color_range,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
            frame_rate,
            sar,
            hdr_format,
        })
    }
}

/// Dolby Vision RPU (Reference Processing Unit) metadata parser.
#[derive(Debug, Clone, PartialEq)]
pub struct DolbyVisionRpuParser;

impl DolbyVisionRpuParser {
    pub fn parse_rpu(raw_rpu: &[u8]) -> Result<DolbyVisionInfo> {
        let unescaped = unescape_nal_unit(raw_rpu);
        let mut slice = unescaped.as_slice();

        // Check for HEVC NAL header
        if slice.len() >= 2 {
            let nal_type = (slice[0] >> 1) & 0x3F;
            if nal_type == 62 || nal_type == 63 {
                slice = &slice[2..];
            }
        }

        let mut r = MsbBitReader::new(slice);
        let rpu_type = r.read_bits(6)?;
        if rpu_type != 2 {
            return Err(MediaInfoError::InvalidData(format!(
                "Unexpected Dolby Vision RPU type {rpu_type}"
            )));
        }

        let _rpu_format = r.read_bits(11)?;
        let vdr_rpu_profile = r.read_bits(4)? as u8;
        let vdr_rpu_level = r.read_bits(4)? as u8;

        let profile = DolbyVisionProfile::from_u8(vdr_rpu_profile);

        let vdr_seq_info_present = r.read_bit()?;
        if vdr_seq_info_present {
            let _chrono = r.read_bits(8)?;
            let _bl_minus8 = r.read_ue().unwrap_or(2);
            let _el_minus8 = r.read_ue().unwrap_or(2);
        }

        Ok(DolbyVisionInfo {
            profile,
            level: vdr_rpu_level,
            rpu_present: true,
            el_present: profile == DolbyVisionProfile::Profile7,
            bl_present: true,
            bl_signal_compatibility_id: Some(vdr_rpu_profile),
            dm_version: Some("v2.9 / v4.0".to_string()),
        })
    }
}
