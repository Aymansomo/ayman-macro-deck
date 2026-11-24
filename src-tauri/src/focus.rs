//! Auto profile per app.
//!
//! Stream Deck style: a profile can be linked to an application, and while that
//! app owns the foreground window the deck switches to that profile. When the
//! app loses the foreground, the deck returns to the profile that was active
//! before the automatic switch, so a game profile never sticks around after the
//! game is closed.
//!
//! A manual profile change is never fought: switching is only evaluated when the
//! foreground application changes.

use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::keyboard::KeyboardManager;
use crate::state::AppCore;

const POLL_INTERVAL: Duration = Duration::from_millis(800);

/// Executable file name of the window in front, e.g. `Cyberpunk2077.exe`.
fn window_exe(window: HWND) -> Option<String> {
    unsafe {
        let mut process_id = 0u32;
        GetWindowThreadProcessId(window, Some(&mut process_id as *mut u32));
        if process_id == 0 {
            return None;
        }
        exe_name_for_pid(process_id)
    }
}

fn exe_name_for_pid(process_id: u32) -> Option<String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut found = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == process_id {
                    let end = entry
                        .szExeFile
                        .iter()
                        .position(|unit| *unit == 0)
                        .unwrap_or(entry.szExeFile.len());
                    found = Some(String::from_utf16_lossy(&entry.szExeFile[..end]));
                    break;
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        found
    }
}

/// Compares a linked app with a running executable by file name, ignoring case
/// and a trailing `.exe`, so `Cyberpunk2077` and `Cyberpunk2077.exe` match.
fn same_app(linked: &str, exe: &str) -> bool {
    fn normalize(value: &str) -> String {
        let trimmed = value.trim();
        let name = Path::new(trimmed)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| trimmed.to_string());
        let name = name.to_ascii_lowercase();
        name.strip_suffix(".exe").unwrap_or(&name).to_string()
    }

    let linked = normalize(linked);
    !linked.is_empty() && linked == normalize(exe)
}

fn switch_to(
    core: &Arc<AppCore>,
    keyboard: &Arc<KeyboardManager>,
    profile_id: &str,
    profile_name: &str,
) {
    core.apply_profile_switch(profile_id);
    if let Err(error) = core.persist() {
        crate::logging::warn(format!(
            "auto profile: cannot persist switch: {}",
            error
        ));
    }
    if let Err(error) = keyboard.reload() {
        crate::logging::warn(format!(
            "auto profile: cannot refresh shortcuts: {}",
            error
        ));
    }
    crate::logging::info(format!(
        "auto profile -> {} ({}), app in front",
        profile_name, profile_id
    ));
    core.emit_state(&keyboard.status());
}

/// Returns to the profile that was active before an automatic switch.
fn revert(
    core: &Arc<AppCore>,
    keyboard: &Arc<KeyboardManager>,
    previous_profile: Option<String>,
) {
    let config = core.config();
    let fallback = previous_profile
        .filter(|id| config.profile(id).is_some())
        .or_else(|| {
            config
                .profile_by_name("Normal")
                .map(|profile| profile.id.clone())
        });

    if let Some(id) = fallback {
        if id != config.active_profile_id {
            let name = config
                .profile(&id)
                .map(|profile| profile.name.clone())
                .unwrap_or_default();
            switch_to(core, keyboard, &id, &name);
        }
    }
}

/// Watches the foreground window for as long as the app runs.
///
/// Idle cost is one sleep plus two lock reads: the process list is only
/// snapshot when the feature is on, a profile is linked to an app, and the
/// foreground window actually changed.
pub fn start(core: Arc<AppCore>, keyboard: Arc<KeyboardManager>) {
    thread::spawn(move || {
        let own_exe = std::env::current_exe()
            .ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_string()));

        let mut last_window: Option<isize> = None;
        // Profile the automatic switch moved to, so leaving the app can undo it.
        let mut auto_profile: Option<String> = None;
        // Profile that was active before the automatic switch.
        let mut previous_profile: Option<String> = None;

        loop {
            thread::sleep(POLL_INTERVAL);

            // Nothing to do while the feature is off or no profile is linked:
            // never enumerate processes in that case.
            if !core.auto_switch_enabled() || !core.has_linked_profiles() {
                last_window = None;
                if auto_profile.take().is_some() {
                    revert(&core, &keyboard, previous_profile.take());
                }
                continue;
            }

            let window = unsafe { GetForegroundWindow() };
            let handle = window.0 as isize;
            if last_window == Some(handle) {
                continue;
            }
            last_window = Some(handle);

            let Some(exe) = window_exe(window) else {
                continue;
            };

            // The deck's own window must never change which profile is active.
            if own_exe
                .as_deref()
                .map(|own| same_app(own, &exe))
                .unwrap_or(false)
            {
                continue;
            }

            let config = core.config();
            let linked = config
                .profiles
                .iter()
                .find(|profile| {
                    profile
                        .linked_app
                        .as_deref()
                        .map(|app| same_app(app, &exe))
                        .unwrap_or(false)
                })
                .map(|profile| (profile.id.clone(), profile.name.clone()));

            match linked {
                Some((id, name)) => {
                    if config.active_profile_id == id {
                        continue;
                    }
                    if auto_profile.is_none() {
                        previous_profile = Some(config.active_profile_id.clone());
                    }
                    switch_to(&core, &keyboard, &id, &name);
                    auto_profile = Some(id);
                }
                None => {
                    if auto_profile.take().is_none() {
                        continue;
                    }
                    revert(&core, &keyboard, previous_profile.take());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::same_app;

    #[test]
    fn compares_app_names_leniently() {
        assert!(same_app("Cyberpunk2077.exe", "cyberpunk2077.exe"));
        assert!(same_app("Spotify", "Spotify.exe"));
        assert!(same_app(
            r"C:\Program Files\Spotify\Spotify.exe",
            "Spotify.exe"
        ));
        assert!(!same_app("Spotify.exe", "chrome.exe"));
        assert!(!same_app("", "chrome.exe"));
    }
}
