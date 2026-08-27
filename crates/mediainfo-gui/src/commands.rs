use mediainfo::{MediaInfo, MediaReport, OutputFormat};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub struct CliState {
    pub initial_files: Mutex<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    pub category: String,
    pub field: String,
    pub value_a: String,
    pub value_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonDiff {
    pub file_a: String,
    pub file_b: String,
    pub report_a: MediaReport,
    pub report_b: MediaReport,
    pub differences: Vec<FieldDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub pure_rust: bool,
    pub zero_ffmpeg: bool,
}

#[tauri::command]
pub fn get_initial_files(state: tauri::State<CliState>) -> Vec<String> {
    let files = state.initial_files.lock().unwrap().clone();
    eprintln!("[cmd] get_initial_files -> {:?}", files);
    files
}

#[tauri::command]
pub fn inspect_file(path: String) -> Result<MediaReport, String> {
    eprintln!("[cmd] inspect_file(\"{}\")", path);
    let result = MediaInfo::open_path(&path).map_err(|e| format!("Failed to inspect '{}': {}", path, e));
    match &result {
        Ok(r) => eprintln!("[cmd] inspect_file OK: format={}", r.general.format.display_name()),
        Err(e) => eprintln!("[cmd] inspect_file ERR: {}", e),
    }
    result
}

#[tauri::command]
pub fn inspect_batch(paths: Vec<String>) -> Result<Vec<MediaReport>, String> {
    eprintln!("[cmd] inspect_batch({} files)", paths.len());
    let reports: Vec<MediaReport> = paths
        .par_iter()
        .filter_map(|p| MediaInfo::open_path(p).ok())
        .collect();
    eprintln!("[cmd] inspect_batch -> {} reports", reports.len());
    Ok(reports)
}

#[tauri::command]
pub fn format_report(path: String, format: String) -> Result<String, String> {
    eprintln!("[cmd] format_report(\"{}\", \"{}\")", path, format);
    let report = MediaInfo::open_path(&path).map_err(|e| e.to_string())?;
    let fmt = match format.to_lowercase().as_str() {
        "json" => OutputFormat::Json,
        "xml" => OutputFormat::Xml,
        "csv" => OutputFormat::Csv,
        "html" => OutputFormat::Html,
        _ => OutputFormat::Text,
    };
    fmt.format(&report).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compare_files(path_a: String, path_b: String) -> Result<ComparisonDiff, String> {
    eprintln!("[cmd] compare_files(\"{}\", \"{}\")", path_a, path_b);
    let report_a = MediaInfo::open_path(&path_a).map_err(|e| format!("Error opening file A: {}", e))?;
    let report_b = MediaInfo::open_path(&path_b).map_err(|e| format!("Error opening file B: {}", e))?;

    let mut differences = Vec::new();

    if report_a.general.format != report_b.general.format {
        differences.push(FieldDiff {
            category: "General".to_string(),
            field: "Format".to_string(),
            value_a: report_a.general.format.display_name().to_string(),
            value_b: report_b.general.format.display_name().to_string(),
        });
    }

    if report_a.general.file_size != report_b.general.file_size {
        differences.push(FieldDiff {
            category: "General".to_string(),
            field: "File Size".to_string(),
            value_a: format!("{} bytes", report_a.general.file_size),
            value_b: format!("{} bytes", report_b.general.file_size),
        });
    }

    if let (Some(va), Some(vb)) = (report_a.videos.first(), report_b.videos.first()) {
        if va.format != vb.format {
            differences.push(FieldDiff {
                category: "Video".to_string(),
                field: "Codec".to_string(),
                value_a: va.format.display_name().to_string(),
                value_b: vb.format.display_name().to_string(),
            });
        }
        if va.width != vb.width || va.height != vb.height {
            differences.push(FieldDiff {
                category: "Video".to_string(),
                field: "Resolution".to_string(),
                value_a: format!("{}x{}", va.width, va.height),
                value_b: format!("{}x{}", vb.width, vb.height),
            });
        }
    }

    if let (Some(aa), Some(ab)) = (report_a.audios.first(), report_b.audios.first()) {
        if aa.format != ab.format {
            differences.push(FieldDiff {
                category: "Audio".to_string(),
                field: "Codec".to_string(),
                value_a: aa.format.display_name().to_string(),
                value_b: ab.format.display_name().to_string(),
            });
        }
        if aa.channels != ab.channels {
            differences.push(FieldDiff {
                category: "Audio".to_string(),
                field: "Channels".to_string(),
                value_a: format!("{} ch", aa.channels),
                value_b: format!("{} ch", ab.channels),
            });
        }
    }

    Ok(ComparisonDiff {
        file_a: path_a,
        file_b: path_b,
        report_a,
        report_b,
        differences,
    })
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        pure_rust: true,
        zero_ffmpeg: true,
    }
}
