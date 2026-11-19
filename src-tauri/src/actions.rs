//! ActionEngine — validates and executes button actions natively.
//!
//! No AutoHotkey is involved: applications are launched through the shell, audio
//! devices through SoundVolumeView (same tool the config already points at) and
//! system keys through `SendInput`.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use crate::models::{ButtonAction, MacroButton, Settings};
use crate::state::AppCore;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Upper bound for `delayAfter`, so a bad config cannot stall the worker.
pub const MAX_ACTION_DELAY_MS: u64 = 15_000;

pub const ACTION_TYPES: &[&str] = &[
    "open_application",
    "open_url",
    "switch_steam_account",
    "launch_steam_game",
    "launch_epic_game",
    "change_audio_device",
    "run_command",
    "run_ahk_script",
    "open_folder",
    "toggle_gaming_mode",
    "switch_profile",
    "keyboard_shortcut",
    "volume_up",
    "volume_down",
    "mute",
];

fn required_params(action_type: &str) -> &'static [&'static str] {
    match action_type {
        "open_application" => &["path"],
        "open_url" => &["url"],
        "switch_steam_account" => &["account"],
        "launch_steam_game" => &["appId"],
        "launch_epic_game" => &["uri"],
        "change_audio_device" => &["device"],
        "run_command" => &["command"],
        "run_ahk_script" => &["scriptPath"],
        "open_folder" => &["path"],
        "switch_profile" => &["profile"],
        "keyboard_shortcut" => &["keys"],
        _ => &[],
    }
}

/// Structural check only (used when loading configs, including half-edited ones).
pub fn validate_type(action_type: &str) -> Result<(), String> {
    if ACTION_TYPES.contains(&action_type) {
        Ok(())
    } else {
        Err(format!("Unknown action type \"{}\"", action_type))
    }
}

/// Full check used before executing an action.
pub fn validate(action_type: &str, params: &Value) -> Result<(), String> {
    validate_type(action_type)?;

    let required = required_params(action_type);
    if required.is_empty() {
        return Ok(());
    }

    let Some(object) = params.as_object() else {
        return Err(format!("Action \"{}\" needs parameters", action_type));
    };

    for key in required {
        let present = object
            .get(*key)
            .map(|value| match value {
                Value::String(text) => !text.trim().is_empty(),
                Value::Null => false,
                _ => true,
            })
            .unwrap_or(false);

        if !present {
            return Err(format!(
                "Action \"{}\" is missing \"{}\"",
                action_type, key
            ));
        }
    }

    if action_type == "keyboard_shortcut" {
        let keys = object.get("keys").and_then(|v| v.as_str()).unwrap_or("");
        if parse_shortcut_keys(keys).is_none() {
            return Err(format!("\"{}\" is not a valid key combination", keys));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub enum Outcome {
    Done(String),
    ToggleGaming,
    SwitchProfile(String),
}

impl Outcome {
    pub fn message(&self) -> String {
        match self {
            Outcome::Done(message) => message.clone(),
            Outcome::ToggleGaming => "Toggled gaming mode".to_string(),
            Outcome::SwitchProfile(profile) => format!("Switched to {} profile", profile),
        }
    }
}

/// Executes one action. State-changing actions are returned as an `Outcome` so
/// the caller (state manager) can apply them.
pub fn execute(action_type: &str, params: &Value) -> Result<Outcome, String> {
    match action_type {
        "open_application" => {
            let path = params["path"]
                .as_str()
                .ok_or("missing 'path' parameter")?;
            open_path(path)?;
            Ok(Outcome::Done(format!("Opened: {}", path)))
        }

        "open_url" => {
            let url = params["url"].as_str().ok_or("missing 'url' parameter")?;
            open::that(url).map_err(|e| e.to_string())?;
            Ok(Outcome::Done(format!("Opened URL: {}", url)))
        }

        "switch_steam_account" => {
            let account = params["account"]
                .as_str()
                .ok_or("missing 'account' parameter")?;
            let shortcuts = params["shortcuts"].as_object();
            if let Some(shortcuts) = shortcuts {
                if let Some(path) = shortcuts.get(account).and_then(|v| v.as_str()) {
                    open_path(path)?;

                    // Ported from the old AHK engine: with waitUntilOpen the
                    // action blocks until Steam is running and loginusers.vdf
                    // marks this account as MostRecent, so a following
                    // "launch game" runs under the right profile.
                    let wait_secs = match &params["waitUntilOpen"] {
                        Value::Bool(true) => Some(120u64),
                        Value::Number(number) => number.as_u64().map(|n| n.min(300)),
                        Value::String(text) if matches!(text.as_str(), "true" | "1") => Some(120),
                        _ => None,
                    };
                    if let Some(seconds) = wait_secs {
                        let steam_exe = params["steamExecutable"]
                            .as_str()
                            .filter(|path| !path.trim().is_empty())
                            .unwrap_or(r"C:\Program Files (x86)\Steam\steam.exe");
                        wait_for_steam_account(account, steam_exe, seconds)?;
                    }

                    return Ok(Outcome::Done(format!("Switched Steam account: {}", account)));
                }
            }
            Err(format!(
                "Steam account shortcut not found for: {}",
                account
            ))
        }

        "launch_steam_game" => {
            let app_id = params["appId"]
                .as_str()
                .ok_or("missing 'appId' parameter")?;
            let uri = format!("steam://rungameid/{}", app_id);
            open::that(&uri).map_err(|e| e.to_string())?;
            Ok(Outcome::Done(format!("Launched Steam game: {}", app_id)))
        }

        "launch_epic_game" => {
            let uri = params["uri"].as_str().ok_or("missing 'uri' parameter")?;
            open::that(uri).map_err(|e| e.to_string())?;
            Ok(Outcome::Done("Launched Epic game".to_string()))
        }

        "change_audio_device" => {
            let device = params["device"]
                .as_str()
                .ok_or("missing 'device' parameter")?;
            let svv_path = params["soundVolumeViewPath"]
                .as_str()
                .ok_or("missing 'soundVolumeViewPath' parameter")?;
            for device_type in 0..=2 {
                let _ = StdCommand::new(svv_path)
                    .args(["/SetDefault", device, &device_type.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
            }
            Ok(Outcome::Done(format!("Audio switched to: {}", device)))
        }

        "toggle_gaming_mode" => Ok(Outcome::ToggleGaming),

        "switch_profile" => {
            let profile = params["profile"]
                .as_str()
                .ok_or("missing 'profile' parameter")?;
            Ok(Outcome::SwitchProfile(profile.to_string()))
        }

        "open_folder" => {
            let path = params["path"].as_str().ok_or("missing 'path' parameter")?;
            open::that(path).map_err(|e| e.to_string())?;
            Ok(Outcome::Done(format!("Opened folder: {}", path)))
        }

        "run_command" => {
            let command = params["command"]
                .as_str()
                .ok_or("missing 'command' parameter")?;
            let args: Vec<String> = match &params["args"] {
                Value::Array(values) => values
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                Value::String(text) => text
                    .split(',')
                    .map(|a| a.trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect(),
                _ => vec![],
            };
            StdCommand::new(command)
                .args(&args)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(Outcome::Done(format!("Ran command: {}", command)))
        }

        "run_ahk_script" => {
            let script_path = params["scriptPath"]
                .as_str()
                .ok_or("missing 'scriptPath' parameter")?;
            let ahk_exe = params["ahkExecutable"]
                .as_str()
                .unwrap_or("AutoHotkey.exe");
            StdCommand::new(ahk_exe)
                .arg(script_path)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(Outcome::Done(format!("Ran script: {}", script_path)))
        }

        "keyboard_shortcut" => {
            let keys = params["keys"].as_str().ok_or("missing 'keys' parameter")?;
            let virtual_keys = parse_shortcut_keys(keys)
                .ok_or_else(|| format!("\"{}\" is not a valid key combination", keys))?;
            send_shortcut(&virtual_keys);
            Ok(Outcome::Done(format!("Sent shortcut: {}", keys)))
        }

        "volume_up" => {
            send_media_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_UP);
            Ok(Outcome::Done("Volume up".to_string()))
        }

        "volume_down" => {
            send_media_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_DOWN);
            Ok(Outcome::Done("Volume down".to_string()))
        }

        "mute" => {
            send_media_key(windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_MUTE);
            Ok(Outcome::Done("Mute toggled".to_string()))
        }

        other => Err(format!("Unknown action type: {}", other)),
    }
}

fn open_path(path: &str) -> Result<(), String> {
    // Configs use `explorer.exe shell:AppsFolder\...` for UWP apps, which has to
    // be passed as a command with an argument rather than to ShellExecute.
    let trimmed = path.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("explorer.exe ")
        .map(|_| &trimmed["explorer.exe ".len()..])
    {
        StdCommand::new("explorer.exe")
            .arg(rest)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    open::that(trimmed).map_err(|e| e.to_string())
}

/// True when Steam.exe is currently running.
fn steam_is_running() -> bool {
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return false;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
        let mut found = false;
        while ok {
            let name = String::from_utf16_lossy(&entry.szExeFile)
                .trim_end_matches('\0')
                .to_ascii_lowercase();
            if name == "steam.exe" {
                found = true;
                break;
            }
            ok = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        found
    }
}

/// Reads the quoted value following `"<key>"` in a VDF fragment.
fn quoted_value_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("\"{}\"", key);
    let pos = text.find(&marker)?;
    let rest = text[pos + marker.len()..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// True when `loginusers.vdf` marks `account` as the most recent user.
fn steam_account_active(vdf_text: &str, account: &str) -> bool {
    let mut offset = 0usize;
    while let Some(relative) = vdf_text[offset..].find("\"AccountName\"") {
        let start = offset + relative;
        let block = &vdf_text[start..];
        let block_end = block.find('}').unwrap_or(block.len());
        let block = &block[..block_end];
        let name = quoted_value_after(block, "AccountName").unwrap_or("");
        let most_recent = quoted_value_after(block, "MostRecent").unwrap_or("") == "1";
        if most_recent && name.eq_ignore_ascii_case(account) {
            return true;
        }
        offset = start + "AccountName".len() + 1;
    }
    false
}

/// Polls Steam + `loginusers.vdf` until the target account is actually open.
fn wait_for_steam_account(account: &str, steam_executable: &str, timeout_secs: u64) -> Result<(), String> {
    let Some(steam_dir) = Path::new(steam_executable).parent() else {
        return Err(format!("invalid Steam executable path: {}", steam_executable));
    };
    let vdf_path = steam_dir.join("config").join("loginusers.vdf");

    for _ in 0..timeout_secs {
        let ready = steam_is_running()
            && fs::read_to_string(&vdf_path)
                .map(|text| steam_account_active(&text, account))
                .unwrap_or(false);
        if ready {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(format!("Steam account switch timed out: {}", account))
}

/// Fills parameters the editor leaves implicit (paths and shortcuts that live in
/// `settings`) so a button works without duplicating them per action.
pub fn enrich_params(settings: &Settings, action: &ButtonAction) -> Value {
    let mut params = Value::Object(action.params.clone());

    if let Some(object) = params.as_object_mut() {
        match action.action_type.as_str() {
            "change_audio_device" => {
                let empty = object
                    .get("soundVolumeViewPath")
                    .and_then(|v| v.as_str())
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true);
                if empty {
                    let configured = settings
                        .extra
                        .get("audio")
                        .and_then(|a| a.get("soundVolumeViewPath"))
                        .and_then(|v| v.as_str())
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty());
                    object.insert(
                        "soundVolumeViewPath".into(),
                        Value::String(
                            configured.unwrap_or_else(|| "SoundVolumeView.exe".to_string()),
                        ),
                    );
                }
            }
            "switch_steam_account" => {
                if !object.contains_key("shortcuts") {
                    if let Some(shortcuts) = settings
                        .extra
                        .get("steam")
                        .and_then(|s| s.get("profileShortcuts"))
                    {
                        object.insert("shortcuts".into(), shortcuts.clone());
                    }
                }
                if !object.contains_key("steamExecutable") {
                    if let Some(path) = settings
                        .extra
                        .get("steam")
                        .and_then(|s| s.get("steamExecutable"))
                        .and_then(|v| v.as_str())
                        .filter(|path| !path.trim().is_empty())
                    {
                        object.insert("steamExecutable".into(), Value::String(path.to_string()));
                    }
                }
            }
            "run_ahk_script" => {
                if settings.keyboard.ahk_executable_path.trim().is_empty() {
                    object.remove("ahkExecutable");
                } else {
                    object.insert(
                        "ahkExecutable".into(),
                        Value::String(settings.keyboard.ahk_executable_path.clone()),
                    );
                }
            }
            _ => {}
        }
    }

    params
}

/// True for buttons that change the mode/profile and therefore must not trigger
/// the "one-shot gaming" auto-return.
pub fn is_mode_action(button: &MacroButton) -> bool {
    button
        .actions
        .iter()
        .any(|a| a.action_type == "toggle_gaming_mode" || a.action_type == "switch_profile")
}

/// Runs every action of a button in order, applying state changes through `core`.
pub fn run_button(core: &AppCore, button: &MacroButton) -> Vec<String> {
    let settings = core.settings();
    let mut results = Vec::new();

    for action in &button.actions {
        let params = enrich_params(&settings, action);

        if let Err(error) = validate(&action.action_type, &params) {
            crate::logging::warn(format!("action skipped: {}", error));
            results.push(error);
            continue;
        }

        match execute(&action.action_type, &params) {
            Ok(Outcome::Done(message)) => {
                crate::logging::info(format!("action {}: {}", action.action_type, message));
                results.push(message);
            }
            Ok(Outcome::ToggleGaming) => {
                let gaming = !core.gaming();
                core.set_gaming(gaming);
                results.push(if gaming {
                    "Gaming mode ON".to_string()
                } else {
                    "Gaming mode OFF".to_string()
                });
            }
            Ok(Outcome::SwitchProfile(profile)) => {
                core.apply_profile_switch(&profile);
                if let Err(error) = core.persist() {
                    crate::logging::warn(format!("could not persist profile switch: {}", error));
                }
                results.push(format!("Switched to {} profile", profile));
            }
            Err(error) => {
                crate::logging::error(format!("action {} failed: {}", action.action_type, error));
                results.push(format!("{}: {}", action.action_type, error));
            }
        }

        if let Some(delay) = action.delay_after {
            if delay > 0 {
                thread::sleep(Duration::from_millis(delay.min(MAX_ACTION_DELAY_MS)));
            }
        }
    }

    results
}

fn parse_shortcut_keys(keys: &str) -> Option<Vec<u16>> {
    let names: Vec<String> = keys
        .split('+')
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    names.iter().map(|name| key_name_to_vk(name)).collect()
}

fn send_shortcut(virtual_keys: &[u16]) {
    let mut inputs: Vec<windows::Win32::UI::Input::KeyboardAndMouse::INPUT> = Vec::new();
    for &vk in virtual_keys {
        inputs.push(make_key_input(vk, false));
    }
    for &vk in virtual_keys.iter().rev() {
        inputs.push(make_key_input(vk, true));
    }
    unsafe {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
            &inputs,
            std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32,
        );
    }
}

fn send_media_key(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        SendInput,
    };

    unsafe {
        let down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: KEYEVENTF_EXTENDEDKEY,
                    ..Default::default()
                },
            },
        };
        let up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        };
        let _ = SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
    }
}

fn make_key_input(
    vk: u16,
    key_up: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        VIRTUAL_KEY,
    };

    let extended = matches!(vk, 0x21..=0x2E | 0x5B | 0x5C | 0x6F | 0xA3 | 0xA5);
    let mut flags = if extended {
        KEYEVENTF_EXTENDEDKEY
    } else {
        Default::default()
    };
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

fn key_name_to_vk(name: &str) -> Option<u16> {
    let named = match name {
        "ctrl" | "control" => Some(0x11),
        "alt" | "menu" => Some(0x12),
        "shift" => Some(0x10),
        "win" | "super" | "meta" => Some(0x5B),
        "enter" | "return" => Some(0x0D),
        "tab" => Some(0x09),
        "esc" | "escape" => Some(0x1B),
        "space" | " " => Some(0x20),
        "backspace" => Some(0x08),
        "delete" | "del" => Some(0x2E),
        "insert" | "ins" => Some(0x2D),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" | "pgup" => Some(0x21),
        "pagedown" | "pgdn" => Some(0x22),
        "up" | "arrowup" => Some(0x26),
        "down" | "arrowdown" => Some(0x28),
        "left" | "arrowleft" => Some(0x25),
        "right" | "arrowright" => Some(0x27),
        "printscreen" | "prtsc" => Some(0x2C),
        "capslock" => Some(0x14),
        "numlock" => Some(0x90),
        "scrolllock" => Some(0x91),
        _ => None,
    };
    if named.is_some() {
        return named;
    }

    if let Some(rest) = name.strip_prefix('f') {
        if let Ok(number) = rest.parse::<u16>() {
            if (1..=24).contains(&number) {
                return Some(0x6F + number);
            }
        }
    }

    let bytes = name.as_bytes();
    if bytes.len() == 1 {
        let ch = bytes[0];
        if ch.is_ascii_lowercase() {
            return Some(ch.to_ascii_uppercase() as u16);
        }
        if ch.is_ascii_digit() {
            return Some(ch as u16);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_required_parameters() {
        assert!(validate("open_url", &json!({"url": "https://x"} )).is_ok());
        assert!(validate("open_url", &json!({"url": "   "})).is_err());
        assert!(validate("open_url", &json!({})).is_err());
        assert!(validate("toggle_gaming_mode", &json!({})).is_ok());
        assert!(validate("nope", &json!({})).is_err());
    }

    #[test]
    fn validates_shortcut_combinations() {
        assert!(validate("keyboard_shortcut", &json!({"keys": "Ctrl+Shift+M"})).is_ok());
        assert!(validate("keyboard_shortcut", &json!({"keys": "Ctrl+Banana"})).is_err());
    }

    #[test]
    fn maps_key_names_to_virtual_keys() {
        assert_eq!(key_name_to_vk("m"), Some(0x4D));
        assert_eq!(key_name_to_vk("f5"), Some(0x74));
        assert_eq!(key_name_to_vk("enter"), Some(0x0D));
        assert_eq!(key_name_to_vk("banana"), None);
    }

    #[test]
    fn mode_actions_are_flagged() {
        let button = MacroButton {
            actions: vec![ButtonAction {
                action_type: "switch_profile".into(),
                params: [("profile".to_string(), Value::String("Gaming".into()))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(is_mode_action(&button));

        let plain = MacroButton {
            actions: vec![ButtonAction {
                action_type: "volume_up".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!is_mode_action(&plain));
    }

    #[test]
    fn enrichment_pulls_settings_paths() {
        let mut settings = Settings::default();
        settings
            .extra
            .insert("audio".into(), json!({"soundVolumeViewPath": "E:\\svv.exe"}));

        let action = ButtonAction {
            action_type: "change_audio_device".into(),
            params: [("device".to_string(), Value::String("Speakers".into()))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        let params = enrich_params(&settings, &action);
        assert_eq!(params["soundVolumeViewPath"], "E:\\svv.exe");
        assert_eq!(params["device"], "Speakers");
    }

    #[test]
    fn steam_account_active_reads_loginusers_vdf() {
        let vdf = r#"
"users"
{
    "76561198000000001"
    {
        "AccountName"      "SOMO"
        "PersonaName"      "Ayman"
        "MostRecent"       "0"
    }
    "76561198000000002"
    {
        "AccountName"      "spydey"
        "PersonaName"      "Spy"
        "MostRecent"       "1"
    }
}
"#;
        assert!(steam_account_active(vdf, "spydey"));
        assert!(steam_account_active(vdf, "SPYDEY"));
        assert!(!steam_account_active(vdf, "SOMO"));
        assert!(!steam_account_active(vdf, "missing"));
    }

    #[test]
    fn enrichment_pulls_steam_executable() {
        let mut settings = Settings::default();
        settings
            .extra
            .insert("steam".into(), json!({"steamExecutable": "D:\\Steam\\steam.exe"}));
        let action = ButtonAction {
            action_type: "switch_steam_account".into(),
            params: [("account".to_string(), Value::String("SOMO".into()))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let params = enrich_params(&settings, &action);
        assert_eq!(params["steamExecutable"], "D:\\Steam\\steam.exe");
    }
}
