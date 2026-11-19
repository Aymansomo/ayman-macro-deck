//! Launch-at-sign-in.
//!
//! Uses the per-user Run key (`HKCU\...\CurrentVersion\Run`), so no
//! administrator rights are needed and uninstalling the app leaves no service
//! behind. The stored command is the installed executable, quoted because the
//! install path contains spaces.

use std::os::windows::process::CommandExt;
use std::process::Command as StdCommand;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "AYMAN MACRO DECK";
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Command line Windows should run at sign-in.
fn launch_command() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate the application executable: {}", e))?;
    Ok(format!("\"{}\"", exe.display()))
}

/// Comparable form of a Run value: unquoted, backslashes, lower case.
fn normalize(command: &str) -> String {
    command
        .trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_ascii_lowercase()
}

/// Current Run value, or `None` when the entry does not exist.
fn read_value() -> Result<Option<String>, String> {
    let output = StdCommand::new("reg")
        .args(["query", RUN_KEY, "/v", VALUE_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Cannot read the Windows startup entry: {}", e))?;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let name = VALUE_NAME.to_ascii_lowercase();
    for line in text.lines() {
        let line = line.trim();
        if !line.to_ascii_lowercase().contains(&name) {
            continue;
        }
        if let Some((_, data)) = line.split_once("REG_SZ") {
            let data = data.trim();
            if !data.is_empty() {
                return Ok(Some(data.to_string()));
            }
        }
    }
    Ok(None)
}

/// True only when the entry exists *and* points at this executable, so a
/// reinstall to a new folder reads as off instead of silently stale.
pub fn is_enabled() -> bool {
    let Ok(Some(stored)) = read_value() else {
        return false;
    };
    let Ok(command) = launch_command() else {
        return false;
    };
    normalize(&stored) == normalize(&command)
}

pub fn set_enabled(enabled: bool) -> Result<bool, String> {
    if enabled {
        let command = launch_command()?;
        let output = StdCommand::new("reg")
            .args([
                "add", RUN_KEY, "/v", VALUE_NAME, "/t", "REG_SZ", "/d", &command, "/f",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Cannot write the Windows startup entry: {}", e))?;

        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if detail.is_empty() {
                "Windows refused the startup entry".to_string()
            } else {
                format!("Windows refused the startup entry: {}", detail)
            });
        }

        if !is_enabled() {
            return Err("The startup entry was written but does not match this app".to_string());
        }
        crate::logging::info("start with Windows enabled");
        return Ok(true);
    }

    let output = StdCommand::new("reg")
        .args(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Cannot remove the Windows startup entry: {}", e))?;

    // A missing value makes `reg delete` fail; that is the desired end state.
    let still_enabled = is_enabled();
    if still_enabled {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Windows kept the startup entry".to_string()
        } else {
            format!("Windows kept the startup entry: {}", detail)
        });
    }

    crate::logging::info("start with Windows disabled");
    Ok(false)
}
