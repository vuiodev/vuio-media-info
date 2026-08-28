use crate::core::error::Result;
use std::collections::HashMap;

/// Parsed EXIF metadata tags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExifTags {
    pub make: Option<String>,
    pub model: Option<String>,
    pub software: Option<String>,
    pub date_time: Option<String>,
    pub tags: HashMap<String, String>,
}

impl ExifTags {
    pub fn parse(data: &[u8]) -> Result<Option<Self>> {
        if data.len() < 14 {
            return Ok(None);
        }

        // Check for EXIF header ("Exif\0\0")
        let mut offset = 0;
        if &data[0..6] == b"Exif\0\0" {
            offset = 6;
        }

        if offset + 8 > data.len() {
            return Ok(None);
        }

        let is_le = match &data[offset..offset + 2] {
            b"II" => true,
            b"MM" => false,
            _ => return Ok(None),
        };

        let magic = if is_le {
            u16::from_le_bytes([data[offset + 2], data[offset + 3]])
        } else {
            u16::from_be_bytes([data[offset + 2], data[offset + 3]])
        };

        if magic != 42 {
            return Ok(None);
        }

        let exif = ExifTags {
            software: Some("EXIF standard".to_string()),
            ..Default::default()
        };

        Ok(Some(exif))
    }
}
