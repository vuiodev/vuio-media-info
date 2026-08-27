use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::ApeTag;

/// WavPack (`.wv`, `.wvc`) Container Demuxer.
pub struct WavpackDemuxer;

pub const WAVPACK_MAGIC: [u8; 4] = [b'w', b'v', b'p', b'k'];

const SAMPLE_RATES: [u32; 15] = [
    6000, 8000, 9600, 11025, 12000, 16000, 22050, 24000,
    32000, 44100, 48000, 64000, 88200, 96000, 192000,
];

impl WavpackDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 32 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 32,
                actual: data.len(),
            });
        }

        if !data.starts_with(&WAVPACK_MAGIC) {
            return Err(MediaInfoError::InvalidData("Not a valid WavPack file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::WavPack;
        report.general.file_size = data.len() as u64;

        let version = u16::from_le_bytes([data[8], data[9]]);
        report.general.format_version = Some(format!("{}.{}", (version >> 8) & 0xFF, version & 0xFF));

        let total_samples = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let flags = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

        let bytes_per_sample = ((flags & 0x03) + 1) as u8;
        let bit_depth = bytes_per_sample * 8;
        let is_mono = (flags & (1 << 2)) != 0;
        let is_hybrid = (flags & (1 << 3)) != 0;
        let is_float = (flags & (1 << 8)) != 0;
        let sr_idx = ((flags >> 23) & 0x0F) as usize;

        let sample_rate = if sr_idx < SAMPLE_RATES.len() {
            SAMPLE_RATES[sr_idx]
        } else {
            44100
        };

        let channels = if is_mono { 1 } else { 2 };

        let mut a = AudioTrack::default();
        a.format = AudioCodec::WavPack;
        a.format_info = Some(if is_float {
            "WavPack (32-bit Float)".to_string()
        } else {
            "WavPack".to_string()
        });
        a.compression_mode = Some(if is_hybrid {
            "Lossy (Hybrid)".to_string()
        } else {
            "Lossless".to_string()
        });
        a.channels = channels;
        a.sampling_rate = sample_rate;
        a.bit_depth = Some(bit_depth);
        a.channel_layout = Some(if is_mono {
            AudioChannelLayout::Mono
        } else {
            AudioChannelLayout::Stereo
        });

        if total_samples > 0 && total_samples != 0xFFFFFFFF && sample_rate > 0 {
            let duration_ms = (total_samples as f64 / sample_rate as f64) * 1000.0;
            a.duration_ms = Some(duration_ms);
            report.general.duration_ms = Some(duration_ms);

            let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
            a.bit_rate = Some(br);
            report.general.overall_bitrate = Some(br);
        }

        // Try extracting APEv2 tags
        if let Ok(Some(ape)) = ApeTag::parse(data) {
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
    fn test_wavpack_demuxer() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&WAVPACK_MAGIC);
        data[8..10].copy_from_slice(&0x0410u16.to_le_bytes()); // v4.16
        data[12..16].copy_from_slice(&441000u32.to_le_bytes()); // 10 seconds at 44.1kHz
        // flags: bytes_per_sample = 2 (16-bit, value 1), stereo (0), sr_idx = 9 (44100, 9 << 23)
        let flags: u32 = 1 | (9 << 23);
        data[24..28].copy_from_slice(&flags.to_le_bytes());

        let report = WavpackDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::WavPack);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].channels, 2);
        assert_eq!(report.audios[0].sampling_rate, 44100);
        assert_eq!(report.audios[0].bit_depth, Some(16));
    }
}
