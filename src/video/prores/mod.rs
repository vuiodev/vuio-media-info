use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// The ProRes variants, identified by the sample entry / frame container fourcc.
///
/// Profile, chroma format and bit depth are fixed properties of the variant
/// (SMPTE RDD 36 Annex A), so they are known before any frame is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProResVariant {
    Proxy,
    LT,
    Standard,
    HQ,
    Quad4444,
    Quad4444XQ,
    RawHQ,
    Raw,
    Unknown,
}

impl ProResVariant {
    pub fn from_fourcc(fourcc: &[u8; 4]) -> Self {
        match fourcc {
            b"apco" => Self::Proxy,
            b"apcs" => Self::LT,
            b"apcn" => Self::Standard,
            b"apch" => Self::HQ,
            b"ap4h" => Self::Quad4444,
            b"ap4x" => Self::Quad4444XQ,
            b"aprh" => Self::RawHQ,
            b"aprn" => Self::Raw,
            _ => Self::Unknown,
        }
    }

    /// MediaInfo-style profile string (the "ProRes" prefix lives in the format name).
    pub fn profile_name(&self) -> &'static str {
        match self {
            Self::Proxy => "422 Proxy",
            Self::LT => "422 LT",
            Self::Standard => "422",
            Self::HQ => "422 HQ",
            Self::Quad4444 => "4444",
            Self::Quad4444XQ => "4444 XQ",
            Self::RawHQ => "RAW HQ",
            Self::Raw => "RAW",
            Self::Unknown => "",
        }
    }

    pub fn chroma_subsampling(&self) -> ChromaSubsampling {
        match self {
            Self::Quad4444 | Self::Quad4444XQ => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV422,
        }
    }

    /// 4444 variants encode 12 bits per component; the 422 family encodes 10.
    pub fn bit_depth(&self) -> u8 {
        match self {
            Self::Quad4444 | Self::Quad4444XQ => 12,
            _ => 10,
        }
    }

    /// Only the 4444 family can carry an alpha plane.
    pub fn has_alpha(&self) -> bool {
        matches!(self, Self::Quad4444 | Self::Quad4444XQ)
    }
}

/// Apple ProRes frame header (SMPTE RDD 36 section 8.1).
///
/// Note that bit depth is *not* carried in the frame header: it is fixed by the profile,
/// so use [`ProResVariant::bit_depth`] with the sample entry fourcc.
#[derive(Debug, Clone, PartialEq)]
pub struct ProResHeader {
    pub frame_size: u32,
    pub header_size: u16,
    pub version: u8,
    pub encoder_id: [u8; 4],
    pub width: u32,
    pub height: u32,
    pub chroma_subsampling: ChromaSubsampling,
    pub interlace_mode: u8,
    /// Raw chroma_format field: 2 is 4:2:2 and 3 is 4:4:4; 0 and 1 are reserved.
    pub chroma_format: u8,
    /// Raw aspect_ratio_information field (RDD 36 Table 6).
    pub aspect_ratio_information: u8,
    pub frame_rate: Option<f64>,
    /// Raw alpha_channel_type field: 0 none, 1 8-bit integer, 2 16-bit integer.
    pub alpha_channel_type: u8,
    pub alpha_present: bool,
    pub color_primaries: Option<ColorPrimaries>,
    pub transfer_characteristics: Option<TransferCharacteristics>,
    pub matrix_coefficients: Option<MatrixCoefficients>,
}

impl ProResHeader {
    /// Parses a ProRes frame, which begins with a 4-byte size followed by the `icpf`
    /// frame container signature.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 28 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 28,
                actual: data.len(),
            });
        }

        if &data[4..8] != b"icpf" {
            return Err(MediaInfoError::InvalidData(
                "Not a valid Apple ProRes frame header (missing 'icpf')".to_string(),
            ));
        }

        let frame_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        Self::parse_frame_header(&data[8..], frame_size)
    }

    /// Parses a bare frame header, without the size and `icpf` prefix.
    ///
    /// The Matroska ProRes mapping stores frames with that 8-byte prefix removed, so a
    /// demuxer for those containers has only the header itself to work from.
    pub fn parse_frame_header(header: &[u8], frame_size: u32) -> Result<Self> {
        // Re-attach a synthetic prefix so the field offsets below stay the ones the
        // specification gives, which are relative to the start of the frame.
        let mut data = Vec::with_capacity(8 + header.len());
        data.extend_from_slice(&frame_size.to_be_bytes());
        data.extend_from_slice(b"icpf");
        data.extend_from_slice(header);
        Self::decode(&data, frame_size)
    }

    fn decode(data: &[u8], frame_size: u32) -> Result<Self> {
        if data.len() < 28 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 28,
                actual: data.len(),
            });
        }
        let header_size = u16::from_be_bytes([data[8], data[9]]);
        if header_size < 20 {
            return Err(MediaInfoError::InvalidData(format!(
                "ProRes frame header is too short ({header_size} bytes)"
            )));
        }
        let version = data[11];
        let encoder_id = [data[12], data[13], data[14], data[15]];
        let width = u16::from_be_bytes([data[16], data[17]]) as u32;
        let height = u16::from_be_bytes([data[18], data[19]]) as u32;

        // Byte 20: chroma_format(2) reserved(2) interlace_mode(2) reserved(2)
        let chroma_format = (data[20] >> 6) & 0x03;
        let chroma_subsampling = match chroma_format {
            3 => ChromaSubsampling::YUV444,
            // 0 and 1 are reserved; every shipping profile is 4:2:2 or 4:4:4.
            _ => ChromaSubsampling::YUV422,
        };
        let interlace_mode = (data[20] >> 2) & 0x03;

        // Byte 21: aspect_ratio_information(4) frame_rate_code(4)
        let aspect_ratio_information = (data[21] >> 4) & 0x0F;
        let frame_rate = Self::frame_rate_from_code(data[21] & 0x0F);

        let color_primaries = Some(ColorPrimaries::from_u8(data[22]));
        let transfer_characteristics = Some(TransferCharacteristics::from_u8(data[23]));
        let matrix_coefficients = Some(MatrixCoefficients::from_u8(data[24]));

        // Byte 25: reserved(4) alpha_channel_type(4). Only present in a full-length header.
        let alpha_channel_type = if header_size >= 18 {
            data[25] & 0x0F
        } else {
            0
        };
        let alpha_present = alpha_channel_type != 0;

        Ok(Self {
            frame_size,
            header_size,
            version,
            encoder_id,
            width,
            height,
            chroma_subsampling,
            interlace_mode,
            chroma_format,
            aspect_ratio_information,
            frame_rate,
            alpha_channel_type,
            alpha_present,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }

    pub fn scan_type(&self) -> &'static str {
        match self.interlace_mode {
            0 => "Progressive",
            _ => "Interlaced",
        }
    }

    /// Field order for interlaced frames; `None` when progressive.
    pub fn scan_order(&self) -> Option<&'static str> {
        match self.interlace_mode {
            1 => Some("TFF"),
            2 => Some("BFF"),
            _ => None,
        }
    }

    /// Display aspect ratio signalled in the frame header, if any (RDD 36 Table 6).
    pub fn display_aspect_ratio(&self) -> Option<f64> {
        match self.aspect_ratio_information {
            1 => Some(self.width as f64 / self.height.max(1) as f64),
            2 => Some(4.0 / 3.0),
            3 => Some(16.0 / 9.0),
            _ => None,
        }
    }

    /// Bits per alpha sample, or `None` when the frame carries no alpha plane.
    pub fn alpha_bit_depth(&self) -> Option<u8> {
        match self.alpha_channel_type {
            1 => Some(8),
            2 => Some(16),
            _ => None,
        }
    }

    /// The four-character encoder signature (for example `apl0` for Apple, `Lavc`).
    pub fn encoder_identifier(&self) -> Option<String> {
        let s = String::from_utf8_lossy(&self.encoder_id).trim().to_string();
        (!s.is_empty() && self.encoder_id.iter().all(|b| b.is_ascii_graphic())).then_some(s)
    }

    fn frame_rate_from_code(code: u8) -> Option<f64> {
        Some(match code {
            1 => 24.0 / 1.001,
            2 => 24.0,
            3 => 25.0,
            4 => 30.0 / 1.001,
            5 => 30.0,
            6 => 50.0,
            7 => 60.0 / 1.001,
            8 => 60.0,
            9 => 100.0,
            10 => 120.0 / 1.001,
            11 => 120.0,
            _ => return None,
        })
    }

    pub fn profile_from_fourcc(fourcc: &[u8; 4]) -> &'static str {
        match ProResVariant::from_fourcc(fourcc) {
            ProResVariant::Proxy => "ProRes 422 Proxy",
            ProResVariant::LT => "ProRes 422 LT",
            ProResVariant::Standard => "ProRes 422",
            ProResVariant::HQ => "ProRes 422 HQ",
            ProResVariant::Quad4444 => "ProRes 4444",
            ProResVariant::Quad4444XQ => "ProRes 4444 XQ",
            ProResVariant::RawHQ => "ProRes RAW HQ",
            ProResVariant::Raw => "ProRes RAW",
            ProResVariant::Unknown => "ProRes",
        }
    }
}
