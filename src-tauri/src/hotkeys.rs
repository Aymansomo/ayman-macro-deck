//! Global hotkeys: system-wide key combinations that run a deck button.
//!
//! A small background thread owns the registrations. It re-reads the config
//! whenever [`notify_changed`] is called (every config save) and pumps the
//! thread's message queue for `WM_HOTKEY`, so a press works even when the deck
//! window is closed and another app is in front.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{Emitter, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY};

use crate::keyboard::KeyboardManager;
use crate::models::MacroButton;
use crate::state::AppCore;

static RELOAD: AtomicU64 = AtomicU64::new(1);

/// Asks the hotkey thread to re-read the configuration.
pub fn notify_changed() {
    RELOAD.fetch_add(1, Ordering::SeqCst);
}

struct Binding {
    id: i32,
    modifiers: HOT_KEY_MODIFIERS,
    vk: u32,
    button_id: String,
}

pub fn start(core: Arc<AppCore>) {
    thread::spawn(move || {
        let mut registered: Vec<i32> = Vec::new();
        let mut buttons: HashMap<i32, String> = HashMap::new();
        let mut seen = 0u64;
        let mut next_id = 1000i32;

        loop {
            let version = RELOAD.load(Ordering::SeqCst);
            if version != seen {
                seen = version;
                for id in registered.drain(..) {
                    unsafe {
                        let _ = UnregisterHotKey(None, id);
                    }
                }
                buttons.clear();

                let bindings = collect(&core, &mut next_id);
                for binding in bindings {
                    let registered_ok = unsafe {
                        RegisterHotKey(None, binding.id, binding.modifiers, binding.vk).is_ok()
                    };
                    if registered_ok {
                        registered.push(binding.id);
                        buttons.insert(binding.id, binding.button_id);
                    } else {
                        crate::logging::warn(format!(
                            "hotkey for button {} could not be registered (already used?)",
                            binding.button_id
                        ));
                    }
                }
                if !registered.is_empty() {
                    crate::logging::info(format!(
                        "{} global hotkey(s) active",
                        registered.len()
                    ));
                }
            }

            let mut message = MSG::default();
            unsafe {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    if message.message == WM_HOTKEY {
                        if let Some(button_id) = buttons.get(&(message.wParam.0 as i32)) {
                            run_button(&core, button_id);
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(40));
        }
    });
}

fn collect(core: &AppCore, next_id: &mut i32) -> Vec<Binding> {
    let config = core.config();
    let mut used: HashSet<(u32, u32)> = HashSet::new();
    let mut bindings = Vec::new();

    for profile in &config.profiles {
        for button in profile.all_buttons() {
            let Some(text) = button
                .global_hotkey
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            else {
                continue;
            };

            let Some((modifiers, vk)) = parse_hotkey(text) else {
                crate::logging::warn(format!(
                    "hotkey: \"{}\" on \"{}\" is not a valid combination",
                    text, button.name
                ));
                continue;
            };
            if vk == 0 || modifiers.0 == 0 {
                crate::logging::warn(format!(
                    "hotkey: \"{}\" needs a modifier such as Ctrl or Alt",
                    text
                ));
                continue;
            }
            if !used.insert((modifiers.0, vk)) {
                crate::logging::warn(format!("hotkey: \"{}\" is registered twice", text));
                continue;
            }

            let id = *next_id;
            *next_id += 1;
            bindings.push(Binding {
                id,
                modifiers,
                vk,
                button_id: button.id.clone(),
            });
        }
    }

    bindings
}

/// Parses `Ctrl+Alt+F9` into Windows modifiers and a virtual key.
fn parse_hotkey(text: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut key: Option<u16> = None;

    for part in text.split('+').map(str::trim).filter(|part| !part.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "super" | "meta" => modifiers |= MOD_WIN,
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(crate::actions::key_name_to_vk(other)?);
            }
        }
    }

    Some((modifiers, key? as u32))
}

fn run_button(core: &Arc<AppCore>, button_id: &str) {
    let Some(button) = core.find_button(button_id) else {
        return;
    };
    if !button.enabled {
        return;
    }
    fire(core, &button);
}

fn fire(core: &Arc<AppCore>, button: &MacroButton) {
    crate::logging::info(format!("hotkey -> {} ({})", button.name, button.id));
    let _ = core.app().emit(
        "macro-action-started",
        serde_json::json!({
            "buttonId": button.id,
            "buttonName": button.name,
        }),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::actions::run_button(core, button)
    }));
    let results = match result {
        Ok(results) => results,
        Err(_) => vec!["internal error: action engine panicked".to_string()],
    };

    if let Some(keyboard) = core.app().try_state::<Arc<KeyboardManager>>() {
        let _ = keyboard.reload();
    }

    let failed = results.iter().any(|line| {
        line.contains("failed") || line.contains("missing") || line.contains("cannot")
    });
    let _ = core.app().emit(
        if failed {
            "macro-action-error"
        } else {
            "macro-action-completed"
        },
        serde_json::json!({
            "buttonId": button.id,
            "buttonName": button.name,
            "results": results,
            "source": "hotkey",
        }),
    );

    if let Some(keyboard) = core.app().try_state::<Arc<KeyboardManager>>() {
        core.emit_state(&keyboard.status());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_combinations() {
        let (modifiers, vk) = parse_hotkey("Ctrl+Alt+F9").expect("valid");
        assert_eq!(modifiers.0, MOD_CONTROL.0 | MOD_ALT.0);
        assert_eq!(vk, 0x78);

        let (modifiers, vk) = parse_hotkey("win+shift+m").expect("valid");
        assert_eq!(modifiers.0, MOD_WIN.0 | MOD_SHIFT.0);
        assert_eq!(vk, 0x4D);
    }

    #[test]
    fn rejects_incomplete_combinations() {
        assert!(parse_hotkey("Ctrl").is_none());
        assert!(parse_hotkey("Ctrl+Banana").is_none());
        assert!(parse_hotkey("Ctrl+A+B").is_none());
    }
}
