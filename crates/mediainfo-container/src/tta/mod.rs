use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::{ApeTag, Id3v2Tag};

/// TrueAudio (`.tta`) Container Demuxer.
pub struct TtaDemuxer;

pub const TTA_MAGIC: [u8; 4] = [b'T', b'T', b'A', b'1'];

impl TtaDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 22 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 22,
                actual: data.len(),
            });
        }

        if !data.starts_with(&TTA_MAGIC) {
            return Err(MediaInfoError::InvalidData("Not a valid TrueAudio file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::TrueAudio;
        report.general.file_size = data.len() as u64;

        let audio_format = u16::from_le_bytes([data[4], data[5]]);
        let channels = u16::from_le_bytes([data[6], data[7]]) as u32;
        let bits_per_sample = u16::from_le_bytes([data[8], data[9]]) as u8;
        let sample_rate = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);
        let total_samples = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);

        let mut a = AudioTrack::default();
        a.format = AudioCodec::TTA;
        a.format_info = Some(if audio_format == 2 {
            "TrueAudio (IEEE Float)".to_string()
        } else {
            "TrueAudio".to_string()
        });
        a.compression_mode = Some("Lossless".to_string());
        a.channels = channels;
        a.sampling_rate = sample_rate;
        a.bit_depth = Some(bits_per_sample);
        a.channel_layout = match channels {
            1 => Some(AudioChannelLayout::Mono),
            2 => Some(AudioChannelLayout::Stereo),
            6 => Some(AudioChannelLayout::Surround5_1),
            _ => Some(AudioChannelLayout::Stereo),
        };

        if sample_rate > 0 && total_samples > 0 {
            let duration_ms = (total_samples as f64 / sample_rate as f64) * 1000.0;
            a.duration_ms = Some(duration_ms);
            report.general.duration_ms = Some(duration_ms);

            let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
            a.bit_rate = Some(br);
            report.general.overall_bitrate = Some(br);
        }

        // Try extracting ID3v2 or APEv2 tags
        if let Ok(Some(id3)) = Id3v2Tag::parse(data) {
            report.general.title = id3.title;
            report.general.artist = id3.artist;
            report.general.album = id3.album;
            report.general.recorded_date = id3.date;
            report.general.genre = id3.genre;
            if id3.cover_data.is_some() {
                report.general.cover_art_present = true;
                report.general.cover_mime = id3.cover_mime;
            }
        } else if let Ok(Some(ape)) = ApeTag::parse(data) {
            report.general.title = ape.title;
            report.general.artist = ape.artist;
            report.general.album = ape.album;
            report.general.recorded_date = ape.year;
            report.general.genre = ape.genre;
            if ape.cover_data.is_some() {
                report.general.cover_art_present = true;
                report.general.cover_mime = ape.cover_mime;
            }
        }

        report.audios.push(a);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trueaudio_demuxer() {
        let mut data = vec![0u8; 22];
        data[0..4].copy_from_slice(&TTA_MAGIC);
        data[4..6].copy_from_slice(&1u16.to_le_bytes()); // PCM
        data[6..8].copy_from_slice(&2u16.to_le_bytes()); // 2 ch
        data[8..10].copy_from_slice(&16u16.to_le_bytes()); // 16 bit
        data[10..14].copy_from_slice(&44100u32.to_le_bytes()); // 44.1kHz
        data[14..18].copy_from_slice(&441000u32.to_le_bytes()); // 10 sec

        let report = TtaDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::TrueAudio);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].channels, 2);
        assert_eq!(report.audios[0].sampling_rate, 44100);
        assert_eq!(report.audios[0].bit_depth, Some(16));
    }
}
