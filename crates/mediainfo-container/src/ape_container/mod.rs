use mediainfo_core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};
use mediainfo_tags::ApeTag;

/// Monkey's Audio (`.ape`) Container Demuxer.
pub struct ApeContainerDemuxer;

pub const APE_MAGIC: [u8; 4] = [b'M', b'A', b'C', b' '];

impl ApeContainerDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 8 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 8,
                actual: data.len(),
            });
        }

        if !data.starts_with(&APE_MAGIC) {
            return Err(MediaInfoError::InvalidData("Not a valid Monkey's Audio file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::APE;
        report.general.file_size = data.len() as u64;

        let version = u16::from_le_bytes([data[4], data[5]]);
        report.general.format_version = Some(format!("{:.2}", version as f64 / 1000.0));

        let mut audio_track = AudioTrack::default();
        audio_track.format = AudioCodec::MonkeyAudio;
        audio_track.format_info = Some("Monkey's Audio".to_string());
        audio_track.compression_mode = Some("Lossless".to_string());

        if version >= 3980 && data.len() >= 76 {
            // New header format (APE >= 3.98)
            let descriptor_bytes = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
            let header_offset = descriptor_bytes.max(52);

            if data.len() >= header_offset + 24 {
                let h = &data[header_offset..];
                let compression_level = u16::from_le_bytes([h[0], h[1]]);
                let _format_flags = u16::from_le_bytes([h[2], h[3]]);
                let blocks_per_frame = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as u64;
                let final_frame_blocks = u32::from_le_bytes([h[8], h[9], h[10], h[11]]) as u64;
                let total_frames = u32::from_le_bytes([h[12], h[13], h[14], h[15]]) as u64;
                let bits_per_sample = u16::from_le_bytes([h[16], h[17]]) as u8;
                let channels = u16::from_le_bytes([h[18], h[19]]) as u32;
                let sample_rate = u32::from_le_bytes([h[20], h[21], h[22], h[23]]);

                let comp_str = match compression_level {
                    1000 => "Fast",
                    2000 => "Normal",
                    3000 => "High",
                    4000 => "Extra High",
                    5000 => "Insane",
                    _ => "Lossless",
                };
                audio_track.format_profile = Some(format!("{} (Level {})", comp_str, compression_level));
                audio_track.channels = channels;
                audio_track.sampling_rate = sample_rate;
                audio_track.bit_depth = Some(bits_per_sample);

                audio_track.channel_layout = match channels {
                    1 => Some(AudioChannelLayout::Mono),
                    2 => Some(AudioChannelLayout::Stereo),
                    6 => Some(AudioChannelLayout::Surround5_1),
                    _ => Some(AudioChannelLayout::Stereo),
                };

                if sample_rate > 0 && total_frames > 0 {
                    let total_samples = if total_frames > 1 {
                        (total_frames - 1) * blocks_per_frame + final_frame_blocks
                    } else {
                        final_frame_blocks
                    };
                    let duration_ms = (total_samples as f64 / sample_rate as f64) * 1000.0;
                    audio_track.duration_ms = Some(duration_ms);
                    report.general.duration_ms = Some(duration_ms);

                    let br = ((data.len() as u64 * 8) as f64 / (duration_ms / 1000.0)) as u64;
                    audio_track.bit_rate = Some(br);
                    report.general.overall_bitrate = Some(br);
                }
            }
        } else if data.len() >= 32 {
            // Legacy APE < 3.98
            let compression_level = u16::from_le_bytes([data[6], data[7]]);
            let channels = u16::from_le_bytes([data[10], data[11]]) as u32;
            let sample_rate = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
            let total_frames = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as u64;
            let final_frame_blocks = u32::from_le_bytes([data[28], data[29], data[30], data[31]]) as u64;

            audio_track.channels = channels;
            audio_track.sampling_rate = sample_rate;
            audio_track.bit_depth = Some(16);
            audio_track.channel_layout = Some(if channels == 1 { AudioChannelLayout::Mono } else { AudioChannelLayout::Stereo });
            audio_track.format_profile = Some(format!("Level {}", compression_level));

            if sample_rate > 0 && total_frames > 0 {
                let blocks_per_frame = 9216u64 * 4;
                let total_samples = (total_frames - 1) * blocks_per_frame + final_frame_blocks;
                let duration_ms = (total_samples as f64 / sample_rate as f64) * 1000.0;
                audio_track.duration_ms = Some(duration_ms);
                report.general.duration_ms = Some(duration_ms);
            }
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

        report.audios.push(audio_track);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ape_container_demuxer() {
        let mut data = Vec::new();
        // APE Descriptor
        data.extend_from_slice(&APE_MAGIC);
        data.extend_from_slice(&3990u16.to_le_bytes()); // version 3.99
        data.extend_from_slice(&0u16.to_le_bytes()); // padding
        data.extend_from_slice(&52u32.to_le_bytes()); // descriptor bytes = 52
        data.extend_from_slice(&24u32.to_le_bytes()); // header bytes = 24
        data.extend_from_slice(&[0u8; 36]); // rest of 52-byte descriptor

        // APE Header (24 bytes)
        data.extend_from_slice(&2000u16.to_le_bytes()); // Normal compression
        data.extend_from_slice(&0u16.to_le_bytes()); // format flags
        data.extend_from_slice(&44100u32.to_le_bytes()); // blocks per frame
        data.extend_from_slice(&44100u32.to_le_bytes()); // final frame blocks
        data.extend_from_slice(&10u32.to_le_bytes()); // 10 frames = 10 seconds at 44.1kHz
        data.extend_from_slice(&16u16.to_le_bytes()); // 16 bit
        data.extend_from_slice(&2u16.to_le_bytes()); // stereo
        data.extend_from_slice(&44100u32.to_le_bytes()); // 44.1kHz

        let report = ApeContainerDemuxer::parse_buffer(&data).unwrap();
        assert_eq!(report.general.format, ContainerFormat::APE);
        assert_eq!(report.general.duration_ms, Some(10000.0));
        assert_eq!(report.audios.len(), 1);
        assert_eq!(report.audios[0].channels, 2);
        assert_eq!(report.audios[0].sampling_rate, 44100);
        assert_eq!(report.audios[0].bit_depth, Some(16));
    }
}
