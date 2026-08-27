use mediainfo_core::{
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
            return Err(MediaInfoError::InvalidData("Not a valid Y4M file".to_string()));
        }

        let mut report = MediaReport::new();
        report.general.format = ContainerFormat::Y4M;
        report.general.file_size = data.len() as u64;

        // Find end of header line (0x0A)
        let header_end = data.iter().position(|&b| b == b'\n').unwrap_or(data.len().min(512));
        let header_str = String::from_utf8_lossy(&data[0..header_end]);

        let mut video_track = VideoTrack::default();
        video_track.format = VideoCodec::Raw;
        video_track.format_info = Some("YUV4MPEG2 Raw Uncompressed Video".to_string());
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
                        if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
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
                        if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
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
                    if val.contains("p10") || val.contains("10") {
                        video_track.bit_depth = 10;
                    } else if val.contains("p12") || val.contains("12") {
                        video_track.bit_depth = 12;
                    }
                }
                _ => {}
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
        assert_eq!(report.videos[0].chroma_subsampling, Some(ChromaSubsampling::YUV420));
    }
}
