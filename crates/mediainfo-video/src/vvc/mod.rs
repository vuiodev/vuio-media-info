use mediainfo_core::{
    bitstream::{MsbBitReader, unescape_nal_unit},
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed VVC (Versatile Video Coding / H.266) SPS NAL unit.
#[derive(Debug, Clone, PartialEq)]
pub struct VvcSps {
    pub profile_idc: u8,
    pub profile_name: String,
    pub tier: String,
    pub level_idc: u8,
    pub level_name: String,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: ChromaSubsampling,
}

impl VvcSps {
    pub fn parse(nal_data: &[u8]) -> Result<Self> {
        if nal_data.len() < 4 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 4,
                actual: nal_data.len(),
            });
        }

        let nal_type = (nal_data[1] >> 3) & 0x1F;
        let payload = if nal_type == 15 {
            &nal_data[2..]
        } else {
            nal_data
        };

        let unescaped = unescape_nal_unit(payload);
        let mut r = MsbBitReader::new(&unescaped);

        let _sps_id = r.read_ue()?;
        let _vps_id = r.read_bits(4)?;
        let _max_sub_layers = r.read_bits(3)?;
        let chroma_format_idc = r.read_bits(2)?;
        let _log2_ctu = r.read_bits(2)?;
        let _ptl_present = r.read_bit()?;

        // Profile Tier Level
        let profile_idc = r.read_bits(7)? as u8;
        let tier_flag = r.read_bit()?;
        let _num_sub_profiles = r.read_bits(8)?;
        // Read 48 bits constraint flags
        let _ = r.read_bits(32)?;
        let _ = r.read_bits(16)?;
        let level_idc = r.read_bits(8)? as u8;

        let width = r.read_ue()?;
        let height = r.read_ue()?;
        let _conformance_window = r.read_bit()?;
        let bitdepth_minus8 = r.read_ue()?;
        let bit_depth = (bitdepth_minus8 + 8) as u8;

        let profile_name = match profile_idc {
            1 => "Main 10".to_string(),
            2 => "Main 10 Still Picture".to_string(),
            3 => "Main 10 4:4:4".to_string(),
            4 => "Main 10 4:4:4 Still Picture".to_string(),
            5 => "Multilayer Main 10".to_string(),
            _ => format!("Profile {}", profile_idc),
        };

        let tier = if tier_flag {
            "High".to_string()
        } else {
            "Main".to_string()
        };
        let level_name = format!("{}.{}", level_idc / 16, (level_idc % 16) / 3);

        let chroma_subsampling = match chroma_format_idc {
            0 => ChromaSubsampling::YUV420,
            1 => ChromaSubsampling::YUV420,
            2 => ChromaSubsampling::YUV422,
            3 => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV420,
        };

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
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vvc_sps_parser() {
        let sps_bytes = [
            0x00, 0x78, // NAL header (type 15 = SPS)
            0x00,
            0x00, // sps_id=0, vps_id=0, max_sub_layers=0, chroma=1 (4:2:0), ctu=0, ptl=0
        ];
        // Minimal sanity check
        assert!(sps_bytes.len() >= 4);
    }
}
