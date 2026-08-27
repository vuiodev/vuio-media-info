pub use mediainfo_core::error::{MediaInfoError, Result};
pub use mediainfo_core::models::MediaReport;
pub use mediainfo_core::types::*;
pub use mediainfo_format::OutputFormat;

use mediainfo_container::ContainerParser;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// High-level facade for inspecting media files and generating reports.
pub struct MediaInfo;

impl MediaInfo {
    /// Open and analyze a media file from a local filesystem path.
    /// Employs fast chunked header probing with zero-copy mmap fallback.
    pub fn open_path(path: impl AsRef<Path>) -> Result<MediaReport> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let file_len = file.metadata()?.len();

        if file_len == 0 {
            let mut empty = MediaReport::new();
            empty.general.file_size = 0;
            empty.general.file_path = Some(path_ref.to_string_lossy().to_string());
            return Ok(empty);
        }

        // Fast Header Probe: Read initial chunk (up to 4MB) to parse in-memory in microseconds
        let initial_chunk_size = (file_len as usize).min(4 * 1024 * 1024);
        let mut buffer = vec![0u8; initial_chunk_size];
        file.read_exact(&mut buffer)?;

        let mut report = match ContainerParser::parse(&buffer) {
            Ok(mut rep) if !rep.videos.is_empty() || !rep.audios.is_empty() || rep.general.duration_ms.is_some() => {
                rep.general.file_size = file_len;
                if let Some(dur_ms) = rep.general.duration_ms {
                    if dur_ms > 0.0 && file_len > 0 {
                        rep.general.overall_bitrate =
                            Some(((file_len as f64 * 8.0) / (dur_ms / 1000.0)) as u64);
                    }
                }
                rep
            }
            _ => {
                // If initial probe was incomplete (e.g. MP4 with moov at end of file), fall back to memory-mapped parsing
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                let mut rep = ContainerParser::parse(&mmap)?;
                rep.general.file_size = file_len;
                if let Some(dur_ms) = rep.general.duration_ms {
                    if dur_ms > 0.0 && file_len > 0 {
                        rep.general.overall_bitrate =
                            Some(((file_len as f64 * 8.0) / (dur_ms / 1000.0)) as u64);
                    }
                }
                rep
            }
        };

        report.general.file_path = Some(path_ref.to_string_lossy().to_string());
        if report.general.file_name.is_none() {
            if let Some(file_name) = path_ref.file_name() {
                report.general.file_name = Some(file_name.to_string_lossy().to_string());
            }
        }

        Ok(report)
    }

    /// Open and analyze a media file from an in-memory byte buffer.
    pub fn open_buffer(buffer: &[u8]) -> Result<MediaReport> {
        ContainerParser::parse(buffer)
    }

    /// Open and analyze a media stream from a standard `Read` reader.
    pub fn open_reader<R: Read>(mut reader: R) -> Result<MediaReport> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Self::open_buffer(&buffer)
    }
}
