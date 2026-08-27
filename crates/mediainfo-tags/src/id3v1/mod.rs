/// Parsed ID3v1 / v1.1 tag (128 bytes).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Id3v1Tag {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub comment: Option<String>,
    pub track_number: Option<u8>,
    pub genre_id: Option<u8>,
}

impl Id3v1Tag {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 128 {
            return None;
        }

        let slice = &data[data.len() - 128..];
        if &slice[0..3] != b"TAG" {
            return None;
        }

        let title = Self::decode_str(&slice[3..33]);
        let artist = Self::decode_str(&slice[33..63]);
        let album = Self::decode_str(&slice[63..93]);
        let year = Self::decode_str(&slice[93..97]);

        let mut track_number = None;
        let comment;
        if slice[125] == 0 && slice[126] != 0 {
            // ID3v1.1 with track number
            comment = Self::decode_str(&slice[97..125]);
            track_number = Some(slice[126]);
        } else {
            comment = Self::decode_str(&slice[97..127]);
        }

        let genre_id = Some(slice[127]);

        Some(Self {
            title,
            artist,
            album,
            year,
            comment,
            track_number,
            genre_id,
        })
    }

    fn decode_str(bytes: &[u8]) -> Option<String> {
        let trimmed = bytes.split(|&b| b == 0).next().unwrap_or(bytes);
        let s = String::from_utf8_lossy(trimmed).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}
