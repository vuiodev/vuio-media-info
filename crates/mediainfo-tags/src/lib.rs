pub mod exif;
pub mod id3v1;
pub mod id3v2;
pub mod itunes;
pub mod vorbis;

pub use exif::ExifTags;
pub use id3v1::Id3v1Tag;
pub use id3v2::Id3v2Tag;
pub use itunes::ItunesTags;
pub use vorbis::VorbisComments;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id3v1_tag() {
        let mut data = vec![0u8; 128];
        data[0..3].copy_from_slice(b"TAG");
        data[3..13].copy_from_slice(b"Test Song\0");
        data[33..45].copy_from_slice(b"Test Artist\0");
        data[63..74].copy_from_slice(b"Test Album\0");
        data[93..97].copy_from_slice(b"2026");
        data[125] = 0;
        data[126] = 5; // track 5
        data[127] = 12; // genre

        let tag = Id3v1Tag::parse(&data).unwrap();
        assert_eq!(tag.title.as_deref(), Some("Test Song"));
        assert_eq!(tag.artist.as_deref(), Some("Test Artist"));
        assert_eq!(tag.album.as_deref(), Some("Test Album"));
        assert_eq!(tag.year.as_deref(), Some("2026"));
        assert_eq!(tag.track_number, Some(5));
    }

    #[test]
    fn test_vorbis_comments() {
        let mut data = Vec::new();
        // Vendor length 4, "test"
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(b"test");

        // 2 comments: "TITLE=My Title", "ARTIST=My Artist"
        data.extend_from_slice(&2u32.to_le_bytes());

        let c1 = b"TITLE=My Title";
        data.extend_from_slice(&(c1.len() as u32).to_le_bytes());
        data.extend_from_slice(c1);

        let c2 = b"ARTIST=My Artist";
        data.extend_from_slice(&(c2.len() as u32).to_le_bytes());
        data.extend_from_slice(c2);

        let vorbis = VorbisComments::parse(&data).unwrap();
        assert_eq!(vorbis.vendor, "test");
        assert_eq!(vorbis.title.as_deref(), Some("My Title"));
        assert_eq!(vorbis.artist.as_deref(), Some("My Artist"));
    }
}
