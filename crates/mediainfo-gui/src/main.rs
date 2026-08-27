#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod window_theme;

use commands::CliState;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    eprintln!("[mediainfo-gui] Starting with {} CLI args: {:?}", args.len(), args);

    tauri::Builder::default()
        .manage(CliState {
            initial_files: Mutex::new(args),
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let window = app.get_webview_window("main").expect("main window not found");
            window_theme::apply_native_theme(&window);

            // Enable devtools in debug builds
            #[cfg(debug_assertions)]
            window.open_devtools();

            eprintln!("[mediainfo-gui] Window setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_initial_files,
            commands::start_window_drag,
            commands::inspect_file,
            commands::inspect_batch,
            commands::format_report,
            commands::compare_files,
            commands::get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mediainfo-gui application");
}
