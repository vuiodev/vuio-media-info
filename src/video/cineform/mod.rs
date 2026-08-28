use crate::core::{
    error::{MediaInfoError, Result},
    types::*,
};

/// Parsed GoPro CineForm HD (SMPTE ST 2073) sample header.
#[derive(Debug, Clone, PartialEq)]
pub struct CineFormHeader {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: ChromaSubsampling,
    pub quality: String,
}

impl CineFormHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        let mut width = 1920;
        let mut height = 1080;
        let mut bit_depth = 10;
        let mut chroma_subsampling = ChromaSubsampling::YUV422;
        let mut quality = "Film Scan 1".to_string();

        // CineForm sample parsing: scan for chunk tags
        let mut offset = 0;
        while offset + 4 <= data.len().min(512) {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let val = &data[offset..offset + len];
            offset += len;

            match tag {
                0x0001 if val.len() >= 4 => {
                    width = u16::from_be_bytes([val[0], val[1]]) as u32;
                    height = u16::from_be_bytes([val[2], val[3]]) as u32;
                }
                0x0002 if !val.is_empty() => {
                    bit_depth = if val[0] == 12 { 12 } else { 10 };
                    if val.len() > 1 && val[1] == 3 {
                        chroma_subsampling = ChromaSubsampling::YUV444;
                    }
                }
                0x0003 if !val.is_empty() => {
                    quality = match val[0] {
                        1 => "Low".to_string(),
                        2 => "Medium".to_string(),
                        3 => "High".to_string(),
                        4 => "Film Scan 1".to_string(),
                        5 => "Film Scan 2".to_string(),
                        _ => "Standard".to_string(),
                    };
                }
                _ => {}
            }
        }

        Ok(Self {
            width,
            height,
            bit_depth,
            chroma_subsampling,
            quality,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cineform_parser() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0001u16.to_be_bytes()); // tag 1 (dimensions)
        data.extend_from_slice(&4u16.to_be_bytes()); // len 4
        data.extend_from_slice(&3840u16.to_be_bytes()); // width 3840
        data.extend_from_slice(&2160u16.to_be_bytes()); // height 2160

        let cf = CineFormHeader::parse(&data).unwrap();
        assert_eq!(cf.width, 3840);
        assert_eq!(cf.height, 2160);
    }
}
