use serde::{Deserialize, Serialize};

/// Color Primaries according to ISO/IEC 23091-2 / ITU-T H.273
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ColorPrimaries {
    Reserved0,
    BT709, // Rec. 709 / sRGB
    Unspecified,
    Reserved3,
    BT470M,    // FCC
    BT470BG,   // PAL / SECAM
    SMPTE170M, // NTSC
    SMPTE240M,
    Film,
    BT2020,   // Rec. 2020 / Rec. 2100 (HDR)
    SMPTE428, // CIE 1931 XYZ
    SMPTE431, // DCI-P3 (Theatrical)
    SMPTE432, // Display P3 (D65)
    EBU3213,
    Unknown(u8),
}

impl ColorPrimaries {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Reserved0,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::BT470M,
            5 => Self::BT470BG,
            6 => Self::SMPTE170M,
            7 => Self::SMPTE240M,
            8 => Self::Film,
            9 => Self::BT2020,
            10 => Self::SMPTE428,
            11 => Self::SMPTE431,
            12 => Self::SMPTE432,
            22 => Self::EBU3213,
            other => Self::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BT709 => "BT.709",
            Self::BT2020 => "BT.2020",
            Self::SMPTE431 => "DCI-P3",
            Self::SMPTE432 => "Display P3",
            Self::SMPTE170M => "BT.601 NTSC",
            Self::BT470BG => "BT.601 PAL",
            Self::Unspecified => "Unspecified",
            _ => "Other",
        }
    }
}

/// Transfer Characteristics (Gamma / EOTF) according to ISO/IEC 23091-2 / ITU-T H.273
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum TransferCharacteristics {
    Reserved0,
    BT709,
    Unspecified,
    Reserved3,
    BT470M,
    BT470BG,
    SMPTE170M,
    SMPTE240M,
    Linear,
    Log100,
    Log316,
    IEC61966_2_4,
    BT1361,
    IEC61966_2_1, // sRGB
    BT2020_10,
    BT2020_12,
    SMPTE2084, // PQ (Perceptual Quantizer / HDR10 / Dolby Vision)
    SMPTE428,
    ARIB_STD_B67, // HLG (Hybrid Log-Gamma)
    Unknown(u8),
}

impl TransferCharacteristics {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Reserved0,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::BT470M,
            5 => Self::BT470BG,
            6 => Self::SMPTE170M,
            7 => Self::SMPTE240M,
            8 => Self::Linear,
            9 => Self::Log100,
            10 => Self::Log316,
            11 => Self::IEC61966_2_4,
            12 => Self::BT1361,
            13 => Self::IEC61966_2_1,
            14 => Self::BT2020_10,
            15 => Self::BT2020_12,
            16 => Self::SMPTE2084,
            17 => Self::SMPTE428,
            18 => Self::ARIB_STD_B67,
            other => Self::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BT709 => "BT.709",
            Self::SMPTE2084 => "PQ",
            Self::ARIB_STD_B67 => "HLG",
            Self::IEC61966_2_1 => "sRGB",
            Self::BT2020_10 | Self::BT2020_12 => "BT.2020",
            Self::SMPTE170M => "BT.601",
            Self::Linear => "Linear",
            Self::Unspecified => "Unspecified",
            _ => "Other",
        }
    }
}

/// Matrix Coefficients according to ISO/IEC 23091-2 / ITU-T H.273
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum MatrixCoefficients {
    Identity, // RGB / GBR
    BT709,
    Unspecified,
    Reserved3,
    FCC,
    BT470BG,
    SMPTE170M,
    SMPTE240M,
    YCgCo,
    BT2020_NCL, // BT.2020 non-constant luminance
    BT2020_CL,  // BT.2020 constant luminance
    SMPTE2085,  // YDzDx
    ChromaDerived_NCL,
    ChromaDerived_CL,
    ICTCP, // ICtCp (Dolby Vision / Rec. 2100)
    Unknown(u8),
}

impl MatrixCoefficients {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Identity,
            1 => Self::BT709,
            2 => Self::Unspecified,
            3 => Self::Reserved3,
            4 => Self::FCC,
            5 => Self::BT470BG,
            6 => Self::SMPTE170M,
            7 => Self::SMPTE240M,
            8 => Self::YCgCo,
            9 => Self::BT2020_NCL,
            10 => Self::BT2020_CL,
            11 => Self::SMPTE2085,
            12 => Self::ChromaDerived_NCL,
            13 => Self::ChromaDerived_CL,
            14 => Self::ICTCP,
            other => Self::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BT709 => "BT.709",
            Self::BT2020_NCL => "BT.2020 non-constant luminance",
            Self::BT2020_CL => "BT.2020 constant luminance",
            Self::SMPTE170M => "BT.601",
            Self::Identity => "Identity / RGB",
            Self::ICTCP => "ICtCp",
            Self::YCgCo => "YCgCo",
            Self::Unspecified => "Unspecified",
            _ => "Other",
        }
    }
}

/// Chroma Subsampling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChromaSubsampling {
    YUV420,
    YUV422,
    YUV444,
    Monochrome,
    RGB,
    Other,
}

impl ChromaSubsampling {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::YUV420 => "4:2:0",
            Self::YUV422 => "4:2:2",
            Self::YUV444 => "4:4:4",
            Self::Monochrome => "Monochrome",
            Self::RGB => "RGB / 4:4:4",
            Self::Other => "Other",
        }
    }
}

/// Color Dynamic Range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorRange {
    Limited, // 16-235 (8-bit) / 64-940 (10-bit)
    Full,    // 0-255 (8-bit) / 0-1023 (10-bit)
}

impl ColorRange {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Limited => "Limited",
            Self::Full => "Full",
        }
    }
}

/// Mastering Display Color Volume (SMPTE ST 2086 / HDR10)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDisplay {
    pub primary_r: (f64, f64), // (x, y)
    pub primary_g: (f64, f64),
    pub primary_b: (f64, f64),
    pub white_point: (f64, f64),
    pub min_luminance: f64, // cd/m2 (nits)
    pub max_luminance: f64, // cd/m2 (nits)
}

/// Content Light Level Information (CTA-861.3 / HDR10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentLightLevel {
    pub max_cll: u32,  // Maximum Content Light Level (cd/m2)
    pub max_fall: u32, // Maximum Frame-Average Light Level (cd/m2)
}
