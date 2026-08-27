use serde::{Deserialize, Serialize};

/// Frame rate mode (Constant vs Variable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameRateMode {
    Constant,
    Variable,
}

impl FrameRateMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::Variable => "Variable",
        }
    }
}

/// Bitrate mode (CBR vs VBR)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BitrateMode {
    Constant,
    Variable,
}

impl BitrateMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::Variable => "Variable",
        }
    }
}
