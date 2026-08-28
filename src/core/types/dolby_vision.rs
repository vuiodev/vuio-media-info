use serde::{Deserialize, Serialize};

/// Dolby Vision Profile according to Dolby Vision Streams Specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DolbyVisionProfile {
    Profile4, // Dual Layer (BL: SDR, EL: DV, RPU)
    Profile5, // Single Layer proprietary IPT / IPTPQc2 (Streaming standard)
    Profile7, // Dual Layer UHD Blu-ray (BL: HDR10 / Rec.2020 PQ, EL: FEL/MEL, RPU)
    Profile8, // Single Layer cross-compatible (BL: HDR10 or HLG, RPU)
    Profile9, // Single Layer AVC/HEVC SDR/HLG (Mobile / broadcast)
    Unknown(u8),
}

impl DolbyVisionProfile {
    pub fn from_u8(val: u8) -> Self {
        match val {
            4 => Self::Profile4,
            5 => Self::Profile5,
            7 => Self::Profile7,
            8 => Self::Profile8,
            9 => Self::Profile9,
            other => Self::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Profile4 => "dvhe.04 (Profile 4)",
            Self::Profile5 => "dvhe.05 (Profile 5)",
            Self::Profile7 => "dvh1.07 / dvhe.07 (Profile 7)",
            Self::Profile8 => "dvh1.08 / dvhe.08 (Profile 8)",
            Self::Profile9 => "dvav.09 (Profile 9)",
            Self::Unknown(_) => "Unknown Profile",
        }
    }
}

/// Detailed Dolby Vision metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DolbyVisionInfo {
    pub profile: DolbyVisionProfile,
    pub level: u8,
    pub rpu_present: bool,
    pub el_present: bool,
    pub bl_present: bool,
    pub bl_signal_compatibility_id: Option<u8>,
    pub dm_version: Option<String>, // e.g. "v1.0", "v2.9", "v4.0"
}
