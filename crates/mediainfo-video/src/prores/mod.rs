use mediainfo_core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Apple ProRes frame header info.
#[derive(Debug, Clone, PartialEq)]
pub struct ProResHeader {
    pub fourcc: [u8; 4],
    pub profile_name: &'static str,
    pub width: u32,
    pub height: u32,
    pub chroma_subsampling: ChromaSubsampling,
    pub bit_depth: u8,
    pub color_primaries: Option<ColorPrimaries>,
    pub transfer_characteristics: Option<TransferCharacteristics>,
    pub matrix_coefficients: Option<MatrixCoefficients>,
}

impl ProResHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 28 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 28,
                actual: data.len(),
            });
        }

        // Check frame header identifier ('icpf')
        let magic = &data[4..8];
        if magic != b"icpf" {
            return Err(MediaInfoError::InvalidData(
                "Not a valid Apple ProRes frame header (missing 'icpf')".to_string(),
            ));
        }

        let width = u16::from_be_bytes([data[8], data[9]]) as u32;
        let height = u16::from_be_bytes([data[10], data[11]]) as u32;
        let chroma_flags = (data[12] >> 6) & 0x03;

        let chroma_subsampling = match chroma_flags {
            2 => ChromaSubsampling::YUV422,
            3 => ChromaSubsampling::YUV444,
            _ => ChromaSubsampling::YUV422,
        };

        let color_primaries = Some(ColorPrimaries::from_u8(data[14]));
        let transfer_characteristics = Some(TransferCharacteristics::from_u8(data[15]));
        let matrix_coefficients = Some(MatrixCoefficients::from_u8(data[16]));

        Ok(Self {
            fourcc: *b"apcn",
            profile_name: "ProRes 422 Standard",
            width,
            height,
            chroma_subsampling,
            bit_depth: 10,
            color_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }

    pub fn profile_from_fourcc(fourcc: &[u8; 4]) -> &'static str {
        match fourcc {
            b"ap4h" | b"ap4x" => "ProRes 4444 XQ",
            b"ap44" => "ProRes 4444",
            b"apch" => "ProRes 422 HQ",
            b"apcn" => "ProRes 422",
            b"apcs" => "ProRes 422 LT",
            b"apco" => "ProRes 422 Proxy",
            _ => "ProRes",
        }
    }
}
