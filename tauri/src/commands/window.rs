//! Native window opacity for glass-like transparency (ORB-13).

use tauri::{Manager, Runtime, WebviewWindow};

/// Smallest practical native alpha when the user picks 0% (window stays clickable).
const MIN_ALPHA_AT_ZERO_PERCENT: f64 = 0.15;

fn clamp_percent(opacity: u8) -> u8 {
    opacity.min(100)
}

fn percent_to_alpha(percent: u8) -> f64 {
    let percent = clamp_percent(percent);
    if percent == 0 {
        MIN_ALPHA_AT_ZERO_PERCENT
    } else {
        f64::from(percent) / 100.0
    }
}

fn apply_window_effects<R: Runtime>(window: &WebviewWindow<R>, percent: u8) -> Result<(), String> {
    if percent >= 100 {
        window.set_effects(None).map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        use tauri::window::{Effect, EffectState, EffectsBuilder};

        #[cfg(target_os = "windows")]
        let effects = EffectsBuilder::new()
            .effect(Effect::Acrylic)
            .state(EffectState::Active)
            .build();

        #[cfg(target_os = "macos")]
        let effects = EffectsBuilder::new()
            .effect(Effect::HudWindow)
            .state(EffectState::Active)
            .build();

        window.set_effects(effects).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn set_native_opacity<R: Runtime>(window: &WebviewWindow<R>, percent: u8) -> Result<(), String> {
    let percent = clamp_percent(percent);
    let alpha = percent_to_alpha(percent);

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
            LWA_ALPHA, WS_EX_LAYERED,
        };

        let raw_hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let hwnd = raw_hwnd.0 as HWND;

        unsafe {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED as isize);
            let byte = if percent >= 100 {
                255u8
            } else {
                ((alpha * 255.0).round() as u32).clamp(1, 255) as u8
            };
            if SetLayeredWindowAttributes(hwnd, 0, byte, LWA_ALPHA) == 0 {
                return Err("SetLayeredWindowAttributes failed".to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use objc::runtime::Object;
        use objc::{msg_send, sel, sel_impl};

        let ns_window = window.ns_window().map_err(|e| e.to_string())? as *mut Object;
        unsafe {
            let _: () = msg_send![ns_window, setAlphaValue: alpha];
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        use gtk::prelude::WidgetExt;

        let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;
        gtk_window.set_opacity(alpha);
    }

    Ok(())
}

/// Set the main window opacity (0–100). 100 is fully opaque.
#[tauri::command]
pub fn set_window_opacity<R: Runtime>(app: tauri::AppHandle<R>, opacity: u8) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let percent = clamp_percent(opacity);

    set_native_opacity(&window, percent)?;

    if percent < 100 {
        let _ = window.set_shadow(false);
        apply_window_effects(&window, percent)?;
    } else {
        let _ = window.set_effects(None);
        let _ = window.set_shadow(true);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clamp_percent, percent_to_alpha};

    #[test]
    fn clamp_percent_bounds() {
        assert_eq!(clamp_percent(0), 0);
        assert_eq!(clamp_percent(50), 50);
        assert_eq!(clamp_percent(150), 100);
    }

    #[test]
    fn percent_to_alpha_scales() {
        assert!((percent_to_alpha(100) - 1.0).abs() < f64::EPSILON);
        assert!((percent_to_alpha(50) - 0.5).abs() < f64::EPSILON);
        assert!((percent_to_alpha(0) - 0.15).abs() < f64::EPSILON);
    }
}
