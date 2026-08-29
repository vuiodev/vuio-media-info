//! # VuIO Media Info
//!
//! A fast, memory-safe, pure Rust library and CLI for exhaustive media metadata inspection and reporting.
//!
//! Provides zero-copy bitstream parsing, container demuxing, and multi-format reporting
//! for video, audio, image, and subtitle formats (ISOBMFF/MP4, Matroska/MKV, RIFF/WAV/AVI,
//! MPEG-TS, FLAC, AAC, MP3, Opus, Dolby Digital/Atmos, DTS, and more).
//!
//! ## Quick Start (Library Usage)
//!
//! ```no_run
//! use vuio_media_info::{inspect, MediaReport, OutputFormat};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Inspect a file from disk
//!     let report: MediaReport = inspect("movie.mkv")?;
//!
//!     // 2. Access strongly-typed properties
//!     println!("Container format: {}", report.general.format);
//!     if let Some(dur) = report.general.duration_ms {
//!         println!("Duration: {:.2}s", dur / 1000.0);
//!     }
//!
//!     for (i, video) in report.videos.iter().enumerate() {
//!         println!("Video #{}: {} ({}x{})", i + 1, video.format, video.width, video.height);
//!     }
//!
//!     // 3. Format as standard JSON schema, Text, XML, CSV, or HTML
//!     let json = report.to_json()?;
//!     println!("{}", json);
//!
//!     Ok(())
//! }
//! ```

pub mod audio;
pub mod container;
pub mod core;
pub mod diff;
pub mod format;
pub mod tags;
pub mod video;

// Re-export primary types and models for top-level ergonomics
pub use core::error::{MediaInfoError, Result};
pub use core::models::*;
pub use core::types::*;
pub use diff::{ComparisonDiff, DiffResult, DifferentialTester, FieldDifference, compare_reports};
pub use format::OutputFormat;

use container::ContainerParser;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Convenience function to analyze a media file from a local filesystem path.
///
/// # Example
/// ```no_run
/// use vuio_media_info::inspect;
///
/// let report = inspect("sample.mp4")?;
/// println!("Container: {}", report.general.format);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn inspect(path: impl AsRef<Path>) -> Result<MediaReport> {
    MediaInfo::open_path(path)
}

/// Convenience function to analyze a media file from an in-memory byte slice.
///
/// # Example
/// ```no_run
/// use vuio_media_info::inspect_bytes;
///
/// let bytes = std::fs::read("audio.flac")?;
/// let report = inspect_bytes(&bytes)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn inspect_bytes(bytes: &[u8]) -> Result<MediaReport> {
    MediaInfo::open_buffer(bytes)
}

/// Convenience function to analyze a media stream from any standard `Read` reader.
///
/// # Example
/// ```no_run
/// use vuio_media_info::inspect_reader;
/// use std::io::Cursor;
///
/// let cursor = Cursor::new(vec![0u8; 100]);
/// let report = inspect_reader(cursor)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn inspect_reader(reader: impl Read) -> Result<MediaReport> {
    MediaInfo::open_reader(reader)
}

/// Convenience function to compare two media reports and produce a differential analysis.
pub fn compare(a: &MediaReport, b: &MediaReport) -> ComparisonDiff {
    compare_reports(a, b)
}

/// Convenience function to inspect and compare two media files by their paths.
pub fn compare_files(path_a: impl AsRef<Path>, path_b: impl AsRef<Path>) -> Result<ComparisonDiff> {
    let rep_a = inspect(path_a)?;
    let rep_b = inspect(path_b)?;
    Ok(compare(&rep_a, &rep_b))
}

/// High-level facade for inspecting media files and generating reports.
pub struct MediaInfo;

impl MediaInfo {
    /// Open and analyze a media file from a local filesystem path.
    ///
    /// Employs fast chunked header probing with zero-copy memory-mapped (`memmap2`) fallback.
    ///
    /// # Example
    /// ```no_run
    /// use vuio_media_info::MediaInfo;
    ///
    /// let report = MediaInfo::open_path("sample.mp4")?;
    /// println!("Format: {}", report.general.format);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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

        // Fast Header Probe: read the file in one go when it is small enough to parse
        // in-memory in microseconds.
        //
        // The probe result is only usable when the whole file was read. Demuxers that
        // derive stream sizes and bit rates by walking every cluster, packet or chunk
        // (Matroska, MPEG-TS, MPEG-PS, AVI) would otherwise report totals for the first
        // 4 MB and silently under-report every larger file.
        const PROBE_LIMIT: usize = 4 * 1024 * 1024;
        let initial_chunk_size = (file_len as usize).min(PROBE_LIMIT);
        let complete_read = file_len as usize <= PROBE_LIMIT;
        let mut buffer = vec![0u8; initial_chunk_size];
        file.read_exact(&mut buffer)?;

        let probe = if complete_read {
            ContainerParser::parse(&buffer)
        } else {
            Err(MediaInfoError::UnsupportedFormat(
                "file exceeds the in-memory probe limit".to_string(),
            ))
        };

        let mut report = match probe {
            Ok(mut rep)
                if !rep.videos.is_empty()
                    || !rep.audios.is_empty()
                    || rep.general.duration_ms.is_some() =>
            {
                rep.general.file_size = file_len;
                if let Some(dur_ms) = rep.general.duration_ms
                    && dur_ms > 0.0
                    && file_len > 0
                {
                    rep.general.overall_bitrate =
                        Some(((file_len as f64 * 8.0) / (dur_ms / 1000.0)) as u64);
                }
                rep
            }
            _ => {
                // Larger files, and files whose headers sit past the probe (an MP4 with a
                // trailing moov), are parsed through a memory map so demuxers still only
                // fault in the pages they actually walk.
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                let mut rep = ContainerParser::parse(&mmap)?;
                rep.general.file_size = file_len;
                if let Some(dur_ms) = rep.general.duration_ms
                    && dur_ms > 0.0
                    && file_len > 0
                {
                    rep.general.overall_bitrate =
                        Some(((file_len as f64 * 8.0) / (dur_ms / 1000.0)) as u64);
                }
                rep
            }
        };

        report.general.file_path = Some(path_ref.to_string_lossy().to_string());
        if report.general.file_name.is_none()
            && let Some(file_name) = path_ref.file_name()
        {
            report.general.file_name = Some(file_name.to_string_lossy().to_string());
        }

        Ok(report)
    }

    /// Open and analyze a media file from an in-memory byte buffer.
    ///
    /// # Example
    /// ```no_run
    /// use vuio_media_info::MediaInfo;
    ///
    /// let bytes = std::fs::read("audio.flac")?;
    /// let report = MediaInfo::open_buffer(&bytes)?;
    /// println!("Channels: {:?}", report.audios.first().map(|a| a.channels));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_buffer(buffer: &[u8]) -> Result<MediaReport> {
        ContainerParser::parse(buffer)
    }

    /// Open and analyze a media stream from any standard `Read` reader.
    ///
    /// # Example
    /// ```no_run
    /// use vuio_media_info::MediaInfo;
    /// use std::io::Cursor;
    ///
    /// let cursor = Cursor::new(vec![0u8; 100]);
    /// let report = MediaInfo::open_reader(cursor)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_reader<R: Read>(mut reader: R) -> Result<MediaReport> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Self::open_buffer(&buffer)
    }
}
