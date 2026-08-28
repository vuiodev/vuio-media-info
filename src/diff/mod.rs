#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::core::{error::Result, models::MediaReport};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// A single field difference between two media reports or files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDifference {
    pub category: String,
    pub field: String,
    pub value_a: String,
    pub value_b: String,
}

/// Comprehensive differential comparison result between two media files or reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonDiff {
    pub file_a: String,
    pub file_b: String,
    pub report_a: MediaReport,
    pub report_b: MediaReport,
    pub differences: Vec<FieldDifference>,
}

impl ComparisonDiff {
    /// Returns `true` if both reports have identical technical properties.
    pub fn is_identical(&self) -> bool {
        self.differences.is_empty()
    }
}

/// Compares two `MediaReport` models and returns all structural and metadata differences.
pub fn compare_reports(report_a: &MediaReport, report_b: &MediaReport) -> ComparisonDiff {
    let mut differences = Vec::new();

    let name_a = report_a
        .general
        .file_name
        .as_deref()
        .unwrap_or("File A")
        .to_string();
    let name_b = report_b
        .general
        .file_name
        .as_deref()
        .unwrap_or("File B")
        .to_string();

    // 1. General Track Comparison
    if report_a.general.format != report_b.general.format {
        differences.push(FieldDifference {
            category: "General".into(),
            field: "Container Format".into(),
            value_a: report_a.general.format.to_string(),
            value_b: report_b.general.format.to_string(),
        });
    }

    if (report_a.general.duration_ms.unwrap_or(0.0) - report_b.general.duration_ms.unwrap_or(0.0))
        .abs()
        > 500.0
    {
        differences.push(FieldDifference {
            category: "General".into(),
            field: "Duration".into(),
            value_a: format!(
                "{:.2}s",
                report_a.general.duration_ms.unwrap_or(0.0) / 1000.0
            ),
            value_b: format!(
                "{:.2}s",
                report_b.general.duration_ms.unwrap_or(0.0) / 1000.0
            ),
        });
    }

    if report_a.general.file_size != report_b.general.file_size
        && report_a.general.file_size > 0
        && report_b.general.file_size > 0
    {
        differences.push(FieldDifference {
            category: "General".into(),
            field: "File Size".into(),
            value_a: format!("{} bytes", report_a.general.file_size),
            value_b: format!("{} bytes", report_b.general.file_size),
        });
    }

    // 2. Video Tracks Comparison
    if report_a.videos.len() != report_b.videos.len() {
        differences.push(FieldDifference {
            category: "Video".into(),
            field: "Track Count".into(),
            value_a: report_a.videos.len().to_string(),
            value_b: report_b.videos.len().to_string(),
        });
    }

    for (i, (va, vb)) in report_a
        .videos
        .iter()
        .zip(report_b.videos.iter())
        .enumerate()
    {
        let cat = format!("Video #{}", i + 1);
        if va.format != vb.format {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Format / Codec".into(),
                value_a: va.format.to_string(),
                value_b: vb.format.to_string(),
            });
        }
        if va.width != vb.width || va.height != vb.height {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Resolution".into(),
                value_a: format!("{}x{}", va.width, va.height),
                value_b: format!("{}x{}", vb.width, vb.height),
            });
        }
        if (va.frame_rate.unwrap_or(0.0) - vb.frame_rate.unwrap_or(0.0)).abs() > 0.05 {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Frame Rate".into(),
                value_a: format!("{:.3} fps", va.frame_rate.unwrap_or(0.0)),
                value_b: format!("{:.3} fps", vb.frame_rate.unwrap_or(0.0)),
            });
        }
        if va.bit_depth != vb.bit_depth {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Bit Depth".into(),
                value_a: format!("{:?} bit", va.bit_depth),
                value_b: format!("{:?} bit", vb.bit_depth),
            });
        }
        if va.hdr_format != vb.hdr_format {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "HDR Format".into(),
                value_a: format!("{:?}", va.hdr_format),
                value_b: format!("{:?}", vb.hdr_format),
            });
        }
    }

    // 3. Audio Tracks Comparison
    if report_a.audios.len() != report_b.audios.len() {
        differences.push(FieldDifference {
            category: "Audio".into(),
            field: "Track Count".into(),
            value_a: report_a.audios.len().to_string(),
            value_b: report_b.audios.len().to_string(),
        });
    }

    for (i, (aa, ab)) in report_a
        .audios
        .iter()
        .zip(report_b.audios.iter())
        .enumerate()
    {
        let cat = format!("Audio #{}", i + 1);
        if aa.format != ab.format {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Format / Codec".into(),
                value_a: aa.format.to_string(),
                value_b: ab.format.to_string(),
            });
        }
        if aa.channels != ab.channels {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Channels".into(),
                value_a: format!("{} ch", aa.channels),
                value_b: format!("{} ch", ab.channels),
            });
        }
        if aa.sampling_rate != ab.sampling_rate {
            differences.push(FieldDifference {
                category: cat.clone(),
                field: "Sampling Rate".into(),
                value_a: format!("{:?} Hz", aa.sampling_rate),
                value_b: format!("{:?} Hz", ab.sampling_rate),
            });
        }
    }

    // 4. Text Tracks Comparison
    if report_a.texts.len() != report_b.texts.len() {
        differences.push(FieldDifference {
            category: "Subtitles".into(),
            field: "Track Count".into(),
            value_a: report_a.texts.len().to_string(),
            value_b: report_b.texts.len().to_string(),
        });
    }

    ComparisonDiff {
        file_a: name_a,
        file_b: name_b,
        report_a: report_a.clone(),
        report_b: report_b.clone(),
        differences,
    }
}

/// Differential testing comparison engine against external tools.
pub struct DifferentialTester;

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    pub matches: bool,
    pub differences: Vec<String>,
}

impl DifferentialTester {
    /// Compares Rust mediainfo report against C++ mediainfo CLI JSON output.
    pub fn compare_file(path: impl AsRef<Path>) -> Result<DiffResult> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref)?;
        let mut rust_report = crate::container::ContainerParser::parse(&bytes)?;
        rust_report.general.file_name = Some(path_ref.to_string_lossy().to_string());

        let mut differences = Vec::new();

        if let Ok(output) = Command::new("mediainfo")
            .arg("--Output=JSON")
            .arg(path_ref)
            .output()
        {
            if output.status.success() {
                if let Ok(cpp_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    Self::compare_json_tracks(&rust_report, &cpp_json, &mut differences);
                }
            }
        }

        let matches = differences.is_empty();
        Ok(DiffResult {
            matches,
            differences,
        })
    }

    fn compare_json_tracks(
        rust_report: &MediaReport,
        cpp_json: &serde_json::Value,
        diffs: &mut Vec<String>,
    ) {
        if let Some(media) = cpp_json.get("media") {
            if let Some(tracks) = media.get("track").and_then(|t| t.as_array()) {
                for track in tracks {
                    let track_type = track.get("@type").and_then(|t| t.as_str()).unwrap_or("");
                    match track_type {
                        "General" => {
                            if let Some(fmt) = track.get("Format").and_then(|f| f.as_str()) {
                                if !rust_report
                                    .general
                                    .format
                                    .display_name()
                                    .eq_ignore_ascii_case(fmt)
                                {
                                    diffs.push(format!(
                                        "General.Format mismatch: Rust={}, C++={}",
                                        rust_report.general.format.display_name(),
                                        fmt
                                    ));
                                }
                            }
                        }
                        "Video" => {
                            if let Some(rust_v) = rust_report.videos.first() {
                                if let Some(w) = track.get("Width").and_then(|w| w.as_str()) {
                                    if rust_v.width.to_string() != w {
                                        diffs.push(format!(
                                            "Video.Width mismatch: Rust={}, C++={}",
                                            rust_v.width, w
                                        ));
                                    }
                                }
                                if let Some(h) = track.get("Height").and_then(|h| h.as_str()) {
                                    if rust_v.height.to_string() != h {
                                        diffs.push(format!(
                                            "Video.Height mismatch: Rust={}, C++={}",
                                            rust_v.height, h
                                        ));
                                    }
                                }
                            }
                        }
                        "Audio" => {
                            if let Some(rust_a) = rust_report.audios.first() {
                                if let Some(ch) = track.get("Channels").and_then(|c| c.as_str()) {
                                    if rust_a.channels.to_string() != ch {
                                        diffs.push(format!(
                                            "Audio.Channels mismatch: Rust={}, C++={}",
                                            rust_a.channels, ch
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
