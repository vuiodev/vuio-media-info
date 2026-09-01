#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod window_settings;
mod window_theme;

use commands::CliState;
use std::sync::Mutex;
use tauri::Manager;
use window_settings::SettingsState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    eprintln!(
        "[mediainfo-gui] Starting with {} CLI args: {:?}",
        args.len(),
        args
    );

    tauri::Builder::default()
        .manage(CliState {
            initial_files: Mutex::new(args),
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            let _ = window.set_title("");
            window_theme::apply_native_theme(&window);

            // Initialize settings state and restore saved window size & position
            let settings_state = SettingsState::new(app.handle());
            let current_settings = settings_state.get_settings();
            window_settings::restore_window_state(&window, &current_settings);
            app.manage(settings_state);

            if let Some(icon) = app.default_window_icon() {
                let _ = window.set_icon(icon.clone());
            }

            // Enable devtools in debug builds
            #[cfg(debug_assertions)]
            window.open_devtools();

            eprintln!("[mediainfo-gui] Window setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        if let Some(settings_state) =
                            window.app_handle().try_state::<SettingsState>()
                        {
                            let is_max = window.is_maximized().unwrap_or(false);
                            let is_min = window.is_minimized().unwrap_or(false);
                            if !is_min {
                                let pos = window.outer_position().unwrap_or_default();
                                let size = window.inner_size().unwrap_or_default();
                                if size.width > 0 && size.height > 0 {
                                    settings_state.update_window_geometry(
                                        pos.x,
                                        pos.y,
                                        size.width,
                                        size.height,
                                        is_max,
                                    );
                                }
                            }
                        }
                    }
                    tauri::WindowEvent::CloseRequested { .. } => {
                        if let Some(settings_state) =
                            window.app_handle().try_state::<SettingsState>()
                        {
                            settings_state.save();
                        }
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_initial_files,
            commands::get_supported_extensions,
            commands::scan_folder,
            commands::start_window_drag,
            commands::inspect_file,
            commands::inspect_batch,
            commands::format_report,
            commands::compare_files,
            commands::get_app_info,
            commands::get_app_settings,
            commands::set_remember_window_state,
            commands::reset_window_geometry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running mediainfo-gui application");
}
