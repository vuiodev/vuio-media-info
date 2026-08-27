use mediainfo_core::error::Result;
use std::collections::HashMap;

/// Parsed APEv2 (and APEv1) tag structure.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApeTag {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub track_number: Option<u32>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub composer: Option<String>,
    pub cover_data: Option<Vec<u8>>,
    pub cover_mime: Option<String>,
    pub extra_tags: HashMap<String, String>,
}

impl ApeTag {
    /// Attempt to parse APE tag from either the end of file (footer) or beginning (header).
    pub fn parse(data: &[u8]) -> Result<Option<Self>> {
        if data.len() < 32 {
            return Ok(None);
        }

        // Try finding APETAGEX footer near the end (common case in MPC, APE, WV)
        // Check exact end (data.len() - 32) or last 160 bytes (if ID3v1 is also present after APE)
        let mut footer_pos = None;

        if data.len() >= 32 && &data[data.len() - 32..data.len() - 24] == b"APETAGEX" {
            footer_pos = Some(data.len() - 32);
        } else if data.len() >= 160 && &data[data.len() - 160..data.len() - 152] == b"APETAGEX" {
            // APE tag followed by 128-byte ID3v1 tag
            footer_pos = Some(data.len() - 160);
        } else {
            // Scan last 4096 bytes for APETAGEX
            let search_start = data.len().saturating_sub(4096);
            let search_slice = &data[search_start..];
            for i in (0..search_slice.len().saturating_sub(32)).rev() {
                if &search_slice[i..i + 8] == b"APETAGEX" {
                    footer_pos = Some(search_start + i);
                    break;
                }
            }
        }

        // Also check if APETAGEX is at the very beginning of the data
        if footer_pos.is_none() && data.starts_with(b"APETAGEX") {
            return Self::parse_from_header(data);
        }

        let footer_offset = match footer_pos {
            Some(pos) => pos,
            None => return Ok(None),
        };

        let footer = &data[footer_offset..footer_offset + 32];
        let tag_size =
            u32::from_le_bytes([footer[12], footer[13], footer[14], footer[15]]) as usize;
        let item_count =
            u32::from_le_bytes([footer[16], footer[17], footer[18], footer[19]]) as usize;

        if !(32..=10 * 1024 * 1024).contains(&tag_size) {
            return Ok(None);
        }

        let tag_start = footer_offset.saturating_sub(tag_size.saturating_sub(32));
        if tag_start >= data.len() {
            return Ok(None);
        }

        let items_data = &data[tag_start..footer_offset];
        Self::parse_items(items_data, item_count)
    }

    fn parse_from_header(data: &[u8]) -> Result<Option<Self>> {
        if data.len() < 32 {
            return Ok(None);
        }
        let tag_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let item_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;

        if tag_size < 32 || tag_size > data.len() {
            return Ok(None);
        }

        let items_data = &data[32..tag_size.min(data.len())];
        Self::parse_items(items_data, item_count)
    }

    fn parse_items(data: &[u8], item_count: usize) -> Result<Option<Self>> {
        let mut tag = ApeTag::default();
        let mut offset = 0;
        let mut parsed_count = 0;

        while offset + 8 < data.len() && parsed_count < item_count {
            let value_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            let flags = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            let is_binary = (flags & 0x06) == 0x02;

            offset += 8;
            if offset >= data.len() {
                break;
            }

            // Read null-terminated key
            let key_start = offset;
            while offset < data.len() && data[offset] != 0 {
                offset += 1;
            }
            if offset >= data.len() {
                break;
            }

            let key_str = String::from_utf8_lossy(&data[key_start..offset]).to_string();
            offset += 1; // skip null byte

            if offset + value_len > data.len() {
                break;
            }

            let val_bytes = &data[offset..offset + value_len];
            offset += value_len;
            parsed_count += 1;

            if is_binary {
                if key_str.eq_ignore_ascii_case("Cover Art (Front)")
                    || key_str.eq_ignore_ascii_case("Cover Art")
                    || key_str.eq_ignore_ascii_case("Picture")
                {
                    // Binary cover art format: null-terminated description string followed by image data
                    if let Some(null_idx) = val_bytes.iter().position(|&b| b == 0) {
                        let img_data = &val_bytes[null_idx + 1..];
                        if img_data.len() >= 4 {
                            tag.cover_mime = if img_data.starts_with(&[0xFF, 0xD8, 0xFF]) {
                                Some("image/jpeg".to_string())
                            } else if img_data.starts_with(b"\x89PNG") {
                                Some("image/png".to_string())
                            } else {
                                Some("image/jpeg".to_string())
                            };
                            tag.cover_data = Some(img_data.to_vec());
                        }
                    } else if val_bytes.len() >= 4 {
                        tag.cover_data = Some(val_bytes.to_vec());
                        tag.cover_mime = Some("image/jpeg".to_string());
                    }
                }
            } else {
                let val_str = String::from_utf8_lossy(val_bytes).to_string();
                match key_str.to_ascii_lowercase().as_str() {
                    "title" => tag.title = Some(val_str),
                    "artist" => tag.artist = Some(val_str),
                    "album" => tag.album = Some(val_str),
                    "year" | "date" => tag.year = Some(val_str),
                    "genre" => tag.genre = Some(val_str),
                    "comment" => tag.comment = Some(val_str),
                    "composer" => tag.composer = Some(val_str),
                    "track" | "tracknumber" => {
                        tag.track_number = val_str
                            .split('/')
                            .next()
                            .and_then(|s| s.trim().parse::<u32>().ok());
                    }
                    _ => {
                        tag.extra_tags.insert(key_str, val_str);
                    }
                }
            }
        }

        Ok(Some(tag))
    }
}
