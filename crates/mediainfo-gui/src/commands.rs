use mediainfo::{MediaInfo, MediaReport, OutputFormat};
use mediainfo_core::types::ContainerFormat;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
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
pub fn get_supported_extensions() -> Vec<&'static str> {
    ContainerFormat::all_supported_extensions().to_vec()
}

#[tauri::command]
pub fn get_initial_files(state: tauri::State<CliState>) -> Vec<String> {
    let raw_args = state.initial_files.lock().unwrap().clone();
    let mut resolved_files = Vec::new();

    for arg in raw_args {
        let p = Path::new(&arg);
        if p.is_dir() {
            for entry in jwalk::WalkDir::new(p).sort(true).skip_hidden(true) {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ContainerFormat::is_supported_extension(ext) {
                                if let Some(path_str) = path.to_str() {
                                    resolved_files.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        } else if p.is_file() {
            resolved_files.push(arg);
        }
    }

    eprintln!("[cmd] get_initial_files -> {} files", resolved_files.len());
    resolved_files
}

#[tauri::command]
pub fn scan_folder(folder_path: String) -> Result<Vec<MediaReport>, String> {
    eprintln!("[cmd] scan_folder(\"{}\")", folder_path);
    let p = Path::new(&folder_path);
    if !p.exists() {
        return Err(format!("Folder '{}' does not exist", folder_path));
    }

    let mut file_paths: Vec<String> = Vec::new();
    for entry in jwalk::WalkDir::new(p).sort(true).skip_hidden(true) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ContainerFormat::is_supported_extension(ext) {
                    if let Some(path_str) = path.to_str() {
                        file_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }

    eprintln!("[cmd] scan_folder found {} media files, inspecting in parallel...", file_paths.len());
    let reports: Vec<MediaReport> = file_paths
        .par_iter()
        .filter_map(|path| MediaInfo::open_path(path).ok())
        .collect();

    eprintln!("[cmd] scan_folder completed: {} reports", reports.len());
    Ok(reports)
}

#[tauri::command]
pub fn start_window_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
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
    let rep_a = MediaInfo::open_path(&path_a).map_err(|e| e.to_string())?;
    let rep_b = MediaInfo::open_path(&path_b).map_err(|e| e.to_string())?;

    let mut differences = Vec::new();

    if rep_a.general.format != rep_b.general.format {
        differences.push(FieldDiff {
            category: "General".to_string(),
            field: "Format".to_string(),
            value_a: rep_a.general.format.display_name().to_string(),
            value_b: rep_b.general.format.display_name().to_string(),
        });
    }

    if rep_a.general.file_size != rep_b.general.file_size {
        differences.push(FieldDiff {
            category: "General".to_string(),
            field: "FileSize".to_string(),
            value_a: format!("{} bytes", rep_a.general.file_size),
            value_b: format!("{} bytes", rep_b.general.file_size),
        });
    }

    if rep_a.videos.len() != rep_b.videos.len() {
        differences.push(FieldDiff {
            category: "Video".to_string(),
            field: "StreamCount".to_string(),
            value_a: rep_a.videos.len().to_string(),
            value_b: rep_b.videos.len().to_string(),
        });
    }

    if rep_a.audios.len() != rep_b.audios.len() {
        differences.push(FieldDiff {
            category: "Audio".to_string(),
            field: "StreamCount".to_string(),
            value_a: rep_a.audios.len().to_string(),
            value_b: rep_b.audios.len().to_string(),
        });
    }

    Ok(ComparisonDiff {
        file_a: path_a,
        file_b: path_b,
        report_a: rep_a,
        report_b: rep_b,
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
