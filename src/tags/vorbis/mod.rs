use crate::core::error::{MediaInfoError, Result};
use std::collections::HashMap;

/// Parsed Vorbis Comments block (used in Ogg, FLAC, Matroska).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VorbisComments {
    pub vendor: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub track: Option<String>,
    pub genre: Option<String>,
    pub comments: HashMap<String, String>,
}

impl VorbisComments {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        let mut offset = 0;

        // Vendor length (32-bit LE)
        let vendor_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        offset += 4;

        if offset + vendor_len > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: offset + vendor_len,
                actual: data.len(),
            });
        }

        let vendor = String::from_utf8_lossy(&data[offset..offset + vendor_len]).to_string();
        offset += vendor_len;

        if offset + 4 > data.len() {
            return Err(MediaInfoError::UnexpectedEof {
                expected: offset + 4,
                actual: data.len(),
            });
        }

        let user_comment_list_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let mut comments = HashMap::new();
        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut date = None;
        let mut track = None;
        let mut genre = None;

        for _ in 0..user_comment_list_len {
            if offset + 4 > data.len() {
                break;
            }

            let comment_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + comment_len > data.len() {
                break;
            }

            let comment_str = String::from_utf8_lossy(&data[offset..offset + comment_len]);
            offset += comment_len;

            if let Some((k, v)) = comment_str.split_once('=') {
                let key_upper = k.to_ascii_uppercase();
                let val_str = v.trim().to_string();

                match key_upper.as_str() {
                    "TITLE" => title = Some(val_str.clone()),
                    "ARTIST" => artist = Some(val_str.clone()),
                    "ALBUM" => album = Some(val_str.clone()),
                    "DATE" | "YEAR" => date = Some(val_str.clone()),
                    "TRACKNUMBER" | "TRACK" => track = Some(val_str.clone()),
                    "GENRE" => genre = Some(val_str.clone()),
                    _ => {}
                }

                comments.insert(key_upper, val_str);
            }
        }

        Ok(Self {
            vendor,
            title,
            artist,
            album,
            date,
            track,
            genre,
            comments,
        })
    }
}
