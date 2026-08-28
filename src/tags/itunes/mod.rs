use crate::core::error::Result;
use std::collections::HashMap;

/// Parsed iTunes/QuickTime `ilst` metadata atoms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItunesTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub genre: Option<String>,
    pub encoder: Option<String>,
    pub cover_mime: Option<String>,
    pub cover_data: Option<Vec<u8>>,
    pub extra: HashMap<String, String>,
}

impl ItunesTags {
    /// Parse raw payload of an `ilst` atom.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut tags = ItunesTags::default();
        let mut offset = 0;

        while offset + 8 <= data.len() {
            let item_size = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;

            if item_size < 8 || offset + item_size > data.len() {
                break;
            }

            let item_tag = &data[offset + 4..offset + 8];
            let item_payload = &data[offset + 8..offset + item_size];
            offset += item_size;

            // Inside each ilst item is typically a `data` atom (size 4, tag "data", type 4, locale 4, value)
            if let Some((type_code, val_bytes)) = Self::extract_data_atom(item_payload) {
                let tag_str = String::from_utf8_lossy(item_tag);

                if item_tag == b"covr" {
                    let mime = if type_code == 14 {
                        "image/png"
                    } else {
                        "image/jpeg"
                    };
                    tags.cover_mime = Some(mime.to_string());
                    tags.cover_data = Some(val_bytes.to_vec());
                } else if type_code == 1 || type_code == 0 {
                    let s = String::from_utf8_lossy(val_bytes).trim().to_string();
                    if !s.is_empty() {
                        match tag_str.as_ref() {
                            "©nam" => tags.title = Some(s),
                            "©ART" => tags.artist = Some(s),
                            "©alb" => tags.album = Some(s),
                            "©day" => tags.date = Some(s),
                            "©gen" => tags.genre = Some(s),
                            "©too" => tags.encoder = Some(s),
                            _ => {
                                tags.extra.insert(tag_str.to_string(), s);
                            }
                        }
                    }
                }
            }
        }

        Ok(tags)
    }

    fn extract_data_atom(payload: &[u8]) -> Option<(u32, &[u8])> {
        if payload.len() < 16 {
            return None;
        }

        let size = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        let tag = &payload[4..8];
        if tag != b"data" || size > payload.len() {
            return None;
        }

        let type_code = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
        let value = &payload[16..size];
        Some((type_code, value))
    }
}
