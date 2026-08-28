use serde::{Deserialize, Serialize};

/// Audio Channel Layout definitions (e.g. Stereo, 5.1, 7.1, 7.1.4 Atmos)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioChannelLayout {
    Mono,
    Stereo,
    Surround2_1,
    Surround3_0,
    Surround4_0,
    Surround5_1,
    Surround7_1,
    Spatial5_1_2,
    Spatial7_1_4,
    ObjectBased,
    Custom(String),
}

impl AudioChannelLayout {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Mono => "Mono (1.0)",
            Self::Stereo => "Stereo (2.0)",
            Self::Surround2_1 => "2.1",
            Self::Surround3_0 => "3.0",
            Self::Surround4_0 => "4.0",
            Self::Surround5_1 => "5.1 Surround (L R C LFE Ls Rs)",
            Self::Surround7_1 => "7.1 Surround (L R C LFE Ls Rs Lb Rb)",
            Self::Spatial5_1_2 => "5.1.2 Spatial / Atmos",
            Self::Spatial7_1_4 => "7.1.4 Spatial / Atmos",
            Self::ObjectBased => "Object Based",
            Self::Custom(name) => name.as_str(),
        }
    }
}
