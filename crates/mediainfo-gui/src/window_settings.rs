use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub remember_window_state: bool,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub window_maximized: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            remember_window_state: true,
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            window_maximized: false,
        }
    }
}

pub struct SettingsState {
    pub settings: Mutex<AppSettings>,
    pub config_path: Option<PathBuf>,
}

impl SettingsState {
    pub fn new(app: &AppHandle) -> Self {
        let config_path = get_settings_path(app);
        let settings = if let Some(ref path) = config_path {
            if let Ok(content) = fs::read_to_string(path) {
                serde_json::from_str::<AppSettings>(&content).unwrap_or_default()
            } else {
                AppSettings::default()
            }
        } else {
            AppSettings::default()
        };

        Self {
            settings: Mutex::new(settings),
            config_path,
        }
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_remember_window_state(&self, enabled: bool) -> AppSettings {
        let mut guard = self.settings.lock().unwrap();
        guard.remember_window_state = enabled;
        let updated = guard.clone();
        drop(guard);
        self.save();
        updated
    }

    pub fn reset_window_geometry(&self, window: &WebviewWindow) -> Result<AppSettings, String> {
        let _ = window.unmaximize();
        let _ = window.set_size(PhysicalSize::new(820, 560));
        let _ = window.center();

        let mut guard = self.settings.lock().unwrap();
        guard.window_x = None;
        guard.window_y = None;
        guard.window_width = Some(820);
        guard.window_height = Some(560);
        guard.window_maximized = false;
        let updated = guard.clone();
        drop(guard);
        self.save();
        Ok(updated)
    }

    pub fn save(&self) {
        if let Some(ref path) = self.config_path
            && let Ok(guard) = self.settings.lock()
            && let Ok(json) = serde_json::to_string_pretty(&*guard)
        {
            let _ = fs::write(path, json);
        }
    }

    pub fn update_window_geometry(&self, x: i32, y: i32, width: u32, height: u32, maximized: bool) {
        let mut should_save = false;
        if let Ok(mut guard) = self.settings.lock()
            && guard.remember_window_state
        {
            if !maximized && width >= 300 && height >= 200 {
                guard.window_x = Some(x);
                guard.window_y = Some(y);
                guard.window_width = Some(width);
                guard.window_height = Some(height);
            }
            guard.window_maximized = maximized;
            should_save = true;
        }
        if should_save {
            self.save();
        }
    }
}

pub fn get_settings_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(mut path) = app.path().app_config_dir() {
        let _ = fs::create_dir_all(&path);
        path.push("settings.json");
        Some(path)
    } else {
        None
    }
}

pub fn restore_window_state(window: &WebviewWindow, settings: &AppSettings) {
    if !settings.remember_window_state {
        return;
    }

    if let (Some(w), Some(h)) = (settings.window_width, settings.window_height) {
        let w = w.max(640);
        let h = h.max(420);
        let _ = window.set_size(PhysicalSize::new(w, h));
    }

    if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }

    if settings.window_maximized {
        let _ = window.maximize();
    }
}
