use mediainfo_core::error::Result;
use std::collections::HashMap;

/// Parsed ID3v2 metadata and attached cover art.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Id3v2Tag {
    pub version: (u8, u8),
    pub total_tag_size: usize,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub track: Option<String>,
    pub genre: Option<String>,
    pub encoder: Option<String>,
    pub cover_mime: Option<String>,
    pub cover_data: Option<Vec<u8>>,
    pub extra: HashMap<String, String>,
}

impl Id3v2Tag {
    pub fn parse(data: &[u8]) -> Result<Option<Self>> {
        if data.len() < 10 {
            return Ok(None);
        }

        if &data[0..3] != b"ID3" {
            return Ok(None);
        }

        let major_ver = data[3];
        let minor_ver = data[4];
        let flags = data[5];

        let _unsynchronization = (flags & 0x80) != 0;
        let extended_header = (flags & 0x40) != 0;

        let tag_size = ((data[6] as usize & 0x7F) << 21)
            | ((data[7] as usize & 0x7F) << 14)
            | ((data[8] as usize & 0x7F) << 7)
            | (data[9] as usize & 0x7F);

        let total_size = 10 + tag_size;
        let parse_limit = total_size.min(data.len());

        let mut offset = 10;
        if extended_header && offset + 4 <= parse_limit {
            let ext_size = ((data[offset] as usize & 0x7F) << 21)
                | ((data[offset + 1] as usize & 0x7F) << 14)
                | ((data[offset + 2] as usize & 0x7F) << 7)
                | (data[offset + 3] as usize & 0x7F);
            offset += 4 + ext_size;
        }

        let mut tag = Id3v2Tag {
            version: (major_ver, minor_ver),
            total_tag_size: total_size,
            ..Default::default()
        };

        while offset + 10 <= total_size {
            let frame_id = &data[offset..offset + 4];
            if frame_id[0] == 0 {
                break;
            }

            let frame_size = if major_ver == 4 {
                ((data[offset + 4] as usize & 0x7F) << 21)
                    | ((data[offset + 5] as usize & 0x7F) << 14)
                    | ((data[offset + 6] as usize & 0x7F) << 7)
                    | (data[offset + 7] as usize & 0x7F)
            } else {
                u32::from_be_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]) as usize
            };

            offset += 10;
            if offset + frame_size > total_size {
                break;
            }

            let frame_data = &data[offset..offset + frame_size];
            offset += frame_size;

            let frame_id_str = String::from_utf8_lossy(frame_id);

            match frame_id_str.as_ref() {
                "TIT2" | "TT2" => tag.title = Self::decode_text_frame(frame_data),
                "TPE1" | "TP1" => tag.artist = Self::decode_text_frame(frame_data),
                "TALB" | "TAL" => tag.album = Self::decode_text_frame(frame_data),
                "TYER" | "TDRC" | "TYE" => tag.date = Self::decode_text_frame(frame_data),
                "TRCK" | "TRK" => tag.track = Self::decode_text_frame(frame_data),
                "TCON" | "TCO" => tag.genre = Self::decode_text_frame(frame_data),
                "TSSE" | "TSS" => tag.encoder = Self::decode_text_frame(frame_data),
                "APIC" | "PIC" => {
                    if let Some((mime, img)) = Self::decode_apic_frame(frame_data) {
                        tag.cover_mime = Some(mime);
                        tag.cover_data = Some(img);
                    }
                }
                _ => {
                    if frame_id_str.starts_with('T') {
                        if let Some(val) = Self::decode_text_frame(frame_data) {
                            tag.extra.insert(frame_id_str.to_string(), val);
                        }
                    }
                }
            }
        }

        Ok(Some(tag))
    }

    fn decode_text_frame(data: &[u8]) -> Option<String> {
        if data.is_empty() {
            return None;
        }

        let encoding = data[0];
        let payload = &data[1..];

        let s = match encoding {
            0 => payload.iter().map(|&b| b as char).collect::<String>(),
            1 => {
                if payload.len() < 2 {
                    return None;
                }
                let mut u16_chars = Vec::new();
                let is_be = payload[0] == 0xFE && payload[1] == 0xFF;
                for chunk in payload[2..].chunks_exact(2) {
                    let code = if is_be {
                        u16::from_be_bytes([chunk[0], chunk[1]])
                    } else {
                        u16::from_le_bytes([chunk[0], chunk[1]])
                    };
                    if code == 0 {
                        break;
                    }
                    u16_chars.push(code);
                }
                String::from_utf16_lossy(&u16_chars)
            }
            2 => {
                let mut u16_chars = Vec::new();
                for chunk in payload.chunks_exact(2) {
                    let code = u16::from_be_bytes([chunk[0], chunk[1]]);
                    if code == 0 {
                        break;
                    }
                    u16_chars.push(code);
                }
                String::from_utf16_lossy(&u16_chars)
            }
            3 => String::from_utf8_lossy(payload).to_string(),
            _ => String::from_utf8_lossy(payload).to_string(),
        };

        let trimmed = s.trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn decode_apic_frame(data: &[u8]) -> Option<(String, Vec<u8>)> {
        if data.len() < 5 {
            return None;
        }

        let mut offset = 1;
        let mime_end = data[offset..].iter().position(|&b| b == 0)?;
        let mime = String::from_utf8_lossy(&data[offset..offset + mime_end]).to_string();
        offset += mime_end + 1;

        if offset >= data.len() {
            return None;
        }

        let _pic_type = data[offset];
        offset += 1;

        let desc_end = data[offset..].iter().position(|&b| b == 0)?;
        offset += desc_end + 1;

        if offset < data.len() {
            let img_data = data[offset..].to_vec();
            Some((mime, img_data))
        } else {
            None
        }
    }
}
