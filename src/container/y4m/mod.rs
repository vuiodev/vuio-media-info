use crate::core::{
    error::{MediaInfoError, Result},
    models::*,
    types::*,
};

/// YUV4MPEG2 (`.y4m`) Container Demuxer.
pub struct Y4mDemuxer;

pub const Y4M_MAGIC: [u8; 9] = *b"YUV4MPEG2";

impl Y4mDemuxer {
    pub fn parse_buffer(data: &[u8]) -> Result<MediaReport> {
        if data.len() < 10 {
            return Err(MediaInfoError::UnexpectedEof {
                expected: 10,
                actual: data.len(),
            });
        }

        if !data.starts_with(&Y4M_MAGIC) {
            return Err(MediaInfoError::InvalidData(
                "Not a valid Y4M file".to_string(),
            ));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::Y4M;
        report.general.file_size = data.len() as u64;

        // Find end of header line (0x0A)
        let header_end = data
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(data.len().min(512));
        let header_str = String::from_utf8_lossy(&data[0..header_end]);

        let mut video_track = VideoTrack::default();
        video_track.format = VideoCodec::Other("YUV".to_string());
        video_track.format_info = Some("YUV4MPEG2 Raw Uncompressed Video".to_string());
        video_track.color_space = Some("YUV".to_string());
        video_track.compression_mode = Some("Lossless".to_string());
        video_track.bit_depth = 8;
        video_track.chroma_subsampling = Some(ChromaSubsampling::YUV420);

        for token in header_str.split_whitespace().skip(1) {
            if token.is_empty() {
                continue;
            }
            let key = &token[0..1];
            let val = &token[1..];

            match key {
                "W" => {
                    if let Ok(w) = val.parse::<u32>() {
                        video_track.width = w;
                    }
                }
                "H" => {
                    if let Ok(h) = val.parse::<u32>() {
                        video_track.height = h;
                    }
                }
                "F" => {
                    let parts: Vec<&str> = val.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(num), Ok(den)) =
                            (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                        {
                            if den > 0.0 {
                                video_track.frame_rate = Some(num / den);
                            }
                        }
                    }
                }
                "I" => {
                    video_track.scan_type = Some(match val {
                        "p" => "Progressive".to_string(),
                        "t" | "b" => "Interlaced".to_string(),
                        _ => "Progressive".to_string(),
                    });
                }
                "A" => {
                    let parts: Vec<&str> = val.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(num), Ok(den)) =
                            (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                        {
                            if den > 0.0 {
                                video_track.sample_aspect_ratio = Some(num / den);
                            }
                        }
                    }
                }
                "C" => {
                    if val.contains("422") {
                        video_track.chroma_subsampling = Some(ChromaSubsampling::YUV422);
                    } else if val.contains("444") {
                        video_track.chroma_subsampling = Some(ChromaSubsampling::YUV444);
                    } else if val.contains("mono") {
                        video_track.chroma_subsampling = Some(ChromaSubsampling::Monochrome);
                    }
                    // The depth suffix is written as p10 / p12 / p16.
                    if val.contains("p16") {
                        video_track.bit_depth = 16;
                    } else if val.contains("p12") {
                        video_track.bit_depth = 12;
                    } else if val.contains("p10") {
                        video_track.bit_depth = 10;
                    }
                }
                _ => {}
            }
        }

        // Frames are stored raw, so the frame size follows from the geometry and the
        // sample format, and the bit rate is exact rather than an average.
        let samples_per_pixel = match video_track.chroma_subsampling {
            Some(ChromaSubsampling::YUV444) => 3.0,
            Some(ChromaSubsampling::YUV422) => 2.0,
            Some(ChromaSubsampling::Monochrome) => 1.0,
            _ => 1.5,
        };
        let bytes_per_sample = video_track.bit_depth.div_ceil(8) as f64;
        let frame_bytes = (video_track.width as f64
            * video_track.height as f64
            * samples_per_pixel
            * bytes_per_sample) as u64;

        if frame_bytes > 0 {
            // Each frame is introduced by a "FRAME" magic line.
            let payload = data.len().saturating_sub(header_end + 1) as u64;
            let frame_count = payload / (frame_bytes + 6);
            if frame_count > 0 {
                video_track.frame_count = Some(frame_count);
                video_track.stream_size = Some(frame_count * frame_bytes);
                if let Some(fps) = video_track.frame_rate.filter(|f| *f > 0.0) {
                    let duration_ms = frame_count as f64 / fps * 1000.0;
                    video_track.duration_ms = Some(duration_ms);
                    report.general.duration_ms = Some(duration_ms);
                    video_track.bit_rate = Some((frame_bytes as f64 * 8.0 * fps) as u64);
                    report.general.overall_bitrate =
                        Some((data.len() as f64 * 8.0 / (duration_ms / 1000.0)) as u64);
                }
            }
        }

        report.videos.push(video_track);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_y4m_demuxer() {
        let header = b"YUV4MPEG2 W1920 H1080 F24:1 Ip A1:1 C420p10\nFRAME\n";
        let report = Y4mDemuxer::parse_buffer(header).unwrap();
        assert_eq!(report.general.format, ContainerFormat::Y4M);
        assert_eq!(report.videos[0].width, 1920);
        assert_eq!(report.videos[0].height, 1080);
        assert_eq!(report.videos[0].frame_rate, Some(24.0));
        assert_eq!(report.videos[0].bit_depth, 10);
        assert_eq!(
            report.videos[0].chroma_subsampling,
            Some(ChromaSubsampling::YUV420)
        );
    }
}
