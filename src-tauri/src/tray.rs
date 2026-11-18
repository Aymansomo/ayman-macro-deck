//! TrayManager — keeps the backend and keyboard hook alive when the window is
//! hidden, and exposes pause/resume plus a clean exit.

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

use crate::keyboard::KeyboardManager;
use crate::state::AppCore;

pub fn show_dashboard(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn setup(app: &tauri::AppHandle, core: Arc<AppCore>, keyboard: Arc<KeyboardManager>) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("window icon".into()))?;

    let show_item = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", "Pause Shortcuts", true, None::<&str>)?;
    let resume_item = MenuItem::with_id(app, "resume", "Resume Shortcuts", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &pause_item,
            &resume_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    let menu_core = core.clone();
    let menu_keyboard = keyboard.clone();

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("AYMAN MACRO DECK")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => show_dashboard(app),
            "pause" => {
                menu_keyboard.set_paused(true);
                menu_core.emit_state(&menu_keyboard.status());
            }
            "resume" => {
                menu_keyboard.set_paused(false);
                menu_core.emit_state(&menu_keyboard.status());
            }
            "quit" => {
                crate::logging::info("quit requested from tray");
                if let Some(devices) = app.try_state::<std::sync::Arc<crate::device::RawInputManager>>() {
                    devices.stop();
                }
                menu_keyboard.stop();
                app.exit(0);
            }
            other => crate::logging::warn(format!("unknown tray menu id: {}", other)),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_dashboard(tray.app_handle());
            }
        })
        .build(app)?;

    crate::logging::info("tray ready");
    Ok(())
}
