use tauri::WebviewWindow;

/// Apply platform-native vibrancy / backdrop blur to the application window.
pub fn apply_native_theme(_window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
        let _ = apply_vibrancy(
            _window,
            NSVisualEffectMaterial::Sidebar,
            Some(window_vibrancy::NSVisualEffectState::Active),
            Some(12.0),
        );
    }

    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        // Try Windows 11 Mica first, fallback to Acrylic
        if apply_mica(_window, None).is_err() {
            let _ = apply_acrylic(_window, Some((20, 20, 26, 200)));
        }
    }
}
