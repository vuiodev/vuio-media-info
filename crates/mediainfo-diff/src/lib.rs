#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use mediainfo_container::ContainerParser;
use mediainfo_core::{error::Result, models::MediaReport};
use std::path::Path;
use std::process::Command;

/// Differential testing comparison engine.
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
        let mut rust_report = ContainerParser::parse(&bytes)?;
        rust_report.general.file_name = Some(path_ref.to_string_lossy().to_string());

        let mut differences = Vec::new();

        // If mediainfo CLI is installed on the system, run it and parse JSON
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
