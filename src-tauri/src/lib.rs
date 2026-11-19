//! AYMAN MACRO DECK — native Rust backend.
//!
//! Phase 1 foundation:
//! - `keyboard`   KeyboardManager  global low-level hook, numpad + modifiers
//! - `actions`    ActionEngine     validated, native action execution
//! - `profiles`   ProfileManager   profile CRUD + switching without restart
//! - `state`      StateManager     config, gaming mode, engine status
//! - `tray`       TrayManager      tray, pause/resume, clean exit
//! - `logging`    file log in `%APPDATA%\com.ayman.macrodeck\logs\app.log`

mod actions;
mod autostart;
mod device;
mod keyid;
mod keyboard;
mod logging;
mod models;
mod profiles;
mod server;
mod shortcut;
mod state;
mod tray;

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::mpsc::channel;
use std::sync::Arc;

use serde::Serialize;
use tauri::Manager;

use device::{DeviceKeyEvent, DeviceStatus, KeyboardDevice, RawInputManager};
use keyboard::{KeyboardManager, KeyboardStatus};
use state::AppCore;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) fn config_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("failed to get app config dir");
    fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

// --------------------------------------------------------------------------
// config
// --------------------------------------------------------------------------

#[tauri::command]
fn load_config(core: tauri::State<'_, Arc<AppCore>>) -> Result<String, String> {
    Ok(core.config_json())
}

#[tauri::command]
fn save_config(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    config: String,
) -> Result<(), String> {
    core.replace_config_json(&config)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(())
}

/// Kept for the current editor, which asks for a reload after saving a button.
/// The native engine re-registers shortcuts instead of restarting a script.
#[tauri::command]
fn reload_ahk_script(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    _ahk_executable: Option<String>,
    _script_path: Option<String>,
) -> Result<(), String> {
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(())
}

// --------------------------------------------------------------------------
// actions
// --------------------------------------------------------------------------

#[tauri::command]
fn execute_action(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    action_type: String,
    params: serde_json::Value,
) -> Result<String, String> {
    actions::validate(&action_type, &params)
        .map_err(|e| format!("Invalid action: {}", e))?;

    let outcome = actions::execute(&action_type, &params)?;
    match &outcome {
        actions::Outcome::ToggleGaming => core.set_gaming(!core.gaming()),
        actions::Outcome::SwitchProfile(name) => {
            core.apply_profile_switch(name);
            core.persist()?;
        }
        actions::Outcome::Done(_) => {}
    }

    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    logging::info(format!("execute_action {}: {}", action_type, outcome.message()));
    Ok(outcome.message())
}

/// Runs a button exactly like a physical key press (used to test a macro).
#[tauri::command]
fn trigger_button(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    button_id: String,
) -> Result<serde_json::Value, String> {
    let results = keyboard.dispatch_button(&button_id);
    core.emit_state(&keyboard.status());
    Ok(serde_json::json!({
        "buttonId": button_id,
        "results": results,
        "state": core.state_json(&keyboard.status()),
    }))
}

// --------------------------------------------------------------------------
// state / keyboard
// --------------------------------------------------------------------------

#[tauri::command]
fn get_app_state(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
) -> serde_json::Value {
    core.state_json(&keyboard.status())
}

#[tauri::command]
fn get_gaming_mode(core: tauri::State<'_, Arc<AppCore>>) -> Result<bool, String> {
    Ok(core.gaming())
}

#[tauri::command]
fn set_gaming_mode(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    gaming: bool,
) -> Result<serde_json::Value, String> {
    core.set_gaming(gaming);
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

#[tauri::command]
fn get_keyboard_connected(
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
) -> Result<bool, String> {
    Ok(keyboard.is_running())
}

#[tauri::command]
fn set_shortcuts_paused(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    paused: bool,
) -> Result<serde_json::Value, String> {
    keyboard.set_paused(paused);
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

#[tauri::command]
fn reload_shortcuts(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
) -> Result<serde_json::Value, String> {
    let count = keyboard.reload()?;
    logging::info(format!("reloaded {} shortcut(s)", count));
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

// --------------------------------------------------------------------------
// devices / engine status (Stage 1: real Raw Input identity + isolation)
// --------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MacroEngineStatus {
    device: DeviceStatus,
    keyboard: KeyboardStatus,
}

#[tauri::command]
fn get_keyboard_devices(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Vec<KeyboardDevice> {
    devices.devices()
}

#[tauri::command]
fn refresh_keyboard_devices(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Vec<KeyboardDevice> {
    devices.refresh()
}

#[tauri::command]
fn get_selected_macro_device(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Option<KeyboardDevice> {
    devices.selected()
}

#[tauri::command]
fn select_macro_device(
    device_id: String,
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Result<KeyboardDevice, String> {
    let selected = devices.select(&device_id)?;
    core.emit_state(&keyboard.status());
    Ok(selected)
}

#[tauri::command]
fn clear_macro_device_selection(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Result<(), String> {
    devices.clear_selection();
    core.emit_state(&keyboard.status());
    Ok(())
}

#[tauri::command]
fn start_device_test(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> DeviceStatus {
    devices.start_test();
    devices.status()
}

#[tauri::command]
fn stop_device_test(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> DeviceStatus {
    devices.stop_test();
    devices.status()
}

#[tauri::command]
fn get_device_test_candidate(
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Option<KeyboardDevice> {
    devices.test_candidate()
}

#[tauri::command]
fn confirm_device_test(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Result<Option<KeyboardDevice>, String> {
    let Some(candidate) = devices.test_candidate() else {
        return Err("Press a key on the intended keyboard first".to_string());
    };
    let selected = devices.select(&candidate.id)?;
    core.emit_state(&keyboard.status());
    Ok(Some(selected))
}

#[tauri::command]
fn get_macro_engine_status(
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> MacroEngineStatus {
    MacroEngineStatus {
        device: devices.status(),
        keyboard: keyboard.status(),
    }
}

#[tauri::command]
fn set_macro_engine_running(
    running: bool,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Result<MacroEngineStatus, String> {
    if running {
        // The observer/worker threads have a controlled lifecycle and are not
        // recreated here; start/stop toggles Raw Input capture plus the pause
        // gate so no stale events can execute.
        devices.start()?;
        keyboard.set_paused(false);
    } else {
        keyboard.set_paused(true);
        devices.stop();
    }
    Ok(MacroEngineStatus {
        device: devices.status(),
        keyboard: keyboard.status(),
    })
}

#[tauri::command]
fn set_macro_device_collections(
    include: bool,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> DeviceStatus {
    devices.set_include_collections(include)
}

/// Keypad capture: on = bound deck keys run their macro and never type, off =
/// deck keys type normally again.
#[tauri::command]
fn set_macro_device_suppression(
    suppress: bool,
    devices: tauri::State<'_, Arc<RawInputManager>>,
) -> Result<DeviceStatus, String> {
    devices.set_suppress_legacy(suppress)
}

// --------------------------------------------------------------------------
// tools: logs, config import/export, backups
// --------------------------------------------------------------------------

// --------------------------------------------------------------------------
// startup
// --------------------------------------------------------------------------

#[tauri::command]
fn get_start_with_windows() -> bool {
    autostart::is_enabled()
}

#[tauri::command]
fn set_start_with_windows(enabled: bool) -> Result<bool, String> {
    autostart::set_enabled(enabled)
}

#[tauri::command]
fn open_logs_folder(app: tauri::AppHandle) -> Result<(), String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    open::that(dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_config(core: tauri::State<'_, Arc<AppCore>>) -> Result<String, String> {
    Ok(core.config_json())
}

#[tauri::command]
fn import_config(
    config: String,
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
) -> Result<(), String> {
    core.replace_config_json(&config)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(())
}

#[tauri::command]
fn list_config_backups(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().into_owned();
        let stamp = name.strip_prefix("config.json.bak-").unwrap_or("");
        if !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit()) {
            names.push(name);
        }
    }
    names.sort();
    names.reverse();
    Ok(names)
}

#[tauri::command]
fn restore_config_backup(
    name: String,
    app: tauri::AppHandle,
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
) -> Result<(), String> {
    let stamp = name.strip_prefix("config.json.bak-").unwrap_or("");
    if stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Invalid backup name".to_string());
    }
    let path = config_path(&app)
        .parent()
        .ok_or("cannot resolve config directory")?
        .join(&name);
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    models::AppConfig::from_json(&text)?;

    // Keep the current config recoverable before overwriting it.
    core.backup_current_config()?;
    core.replace_config_json(&text)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(())
}

// --------------------------------------------------------------------------
// profiles
// --------------------------------------------------------------------------

#[tauri::command]
fn list_profiles(
    core: tauri::State<'_, Arc<AppCore>>,
) -> Result<serde_json::Value, String> {
    let profiles = core.config().profiles;
    Ok(serde_json::json!(profiles))
}

#[tauri::command]
fn create_profile(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    name: String,
) -> Result<serde_json::Value, String> {
    let profile = profiles::create(&core.inner().clone(), &name)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(serde_json::json!({ "profile": profile, "state": core.state_json(&keyboard.status()) }))
}

#[tauri::command]
fn rename_profile(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    profile_id: String,
    name: String,
) -> Result<serde_json::Value, String> {
    profiles::rename(&core.inner().clone(), &profile_id, &name)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

#[tauri::command]
fn duplicate_profile(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    profile_id: String,
    name: String,
) -> Result<serde_json::Value, String> {
    let profile = profiles::duplicate(&core.inner().clone(), &profile_id, &name)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(serde_json::json!({ "profile": profile, "state": core.state_json(&keyboard.status()) }))
}

#[tauri::command]
fn delete_profile(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    profile_id: String,
) -> Result<serde_json::Value, String> {
    profiles::delete(&core.inner().clone(), &profile_id)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

#[tauri::command]
fn set_active_profile(
    core: tauri::State<'_, Arc<AppCore>>,
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    profile_id: String,
) -> Result<serde_json::Value, String> {
    profiles::set_active(&core.inner().clone(), &profile_id)?;
    keyboard.reload()?;
    core.emit_state(&keyboard.status());
    Ok(core.state_json(&keyboard.status()))
}

// --------------------------------------------------------------------------
// system helpers used by the editor
// --------------------------------------------------------------------------

#[tauri::command]
fn list_audio_devices(sound_volume_view_path: String) -> Result<Vec<String>, String> {
    let svv_path = if sound_volume_view_path.trim().is_empty() {
        "SoundVolumeView.exe".to_string()
    } else {
        sound_volume_view_path
    };

    let out_file = std::env::temp_dir().join(format!("svv_{}.csv", std::process::id()));
    let out_path = out_file
        .to_str()
        .ok_or("invalid temp path")?
        .to_string();

    let status = StdCommand::new(&svv_path)
        .args(["/scomma", &out_path])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("failed to run SoundVolumeView: {}", e))?;

    if !status.success() {
        return Ok(fallback_audio_devices());
    }

    let mut content = fs::read_to_string(&out_file).unwrap_or_default();
    let _ = fs::remove_file(&out_file);

    if content.starts_with('\u{FEFF}') {
        content = content.trim_start_matches('\u{FEFF}').to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return Ok(fallback_audio_devices());
    }

    let parse_csv_line = |line: &str| -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
        fields.push(current.trim().to_string());
        fields
    };

    let header = parse_csv_line(lines[0]);
    let name_idx = header.iter().position(|h| h == "Name");
    let type_idx = header.iter().position(|h| h == "Type");
    let dir_idx = header.iter().position(|h| h == "Direction");

    let (Some(name_idx), Some(type_idx), Some(dir_idx)) = (name_idx, type_idx, dir_idx) else {
        return Ok(fallback_audio_devices());
    };

    let mut devices: Vec<String> = Vec::new();
    for line in lines.iter().skip(1) {
        let fields = parse_csv_line(line);
        if fields.len() <= dir_idx {
            continue;
        }
        let is_device = fields[type_idx].eq_ignore_ascii_case("Device");
        let is_render = fields[dir_idx].eq_ignore_ascii_case("Render");
        if is_device && is_render {
            if let Some(name) = fields.get(name_idx) {
                if !name.is_empty() && !devices.contains(name) {
                    devices.push(name.clone());
                }
            }
        }
    }

    if devices.is_empty() {
        Ok(fallback_audio_devices())
    } else {
        Ok(devices)
    }
}

fn fallback_audio_devices() -> Vec<String> {
    let output = StdCommand::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance -ClassName Win32_SoundDevice | Select-Object -ExpandProperty Name",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[tauri::command]
fn list_lnk_files(folder: String) -> Result<Vec<String>, String> {
    let folder = folder.trim();
    if folder.is_empty() {
        return Ok(Vec::new());
    }
    let mut result: Vec<String> = Vec::new();
    match std::fs::read_dir(folder) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_lnk = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(false);
                if !is_lnk {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    result.push(format!("{}\\{}", folder.trim_end_matches('\\'), name));
                }
            }
        }
        Err(e) => return Err(format!("Cannot read folder: {}", e)),
    }
    result.sort();
    Ok(result)
}

// --------------------------------------------------------------------------
// application picker (editor helper)
// --------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationInfo {
    path: String,
    name: String,
    icon_path: Option<String>,
}

/// Returns a chosen executable's display name and a cached PNG of its icon.
/// Icon extraction uses Windows' own shell icon provider via System.Drawing so
/// `.exe` and `.lnk` files resolve to the same image Explorer shows.
#[tauri::command]
fn get_application_info(app: tauri::AppHandle, path: String) -> Result<ApplicationInfo, String> {
    let source = PathBuf::from(&path);
    if !source.is_file() {
        return Err(format!("File not found: {}", path));
    }

    let name = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Application")
        .to_string();

    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("images");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe_name = if safe_name.trim_matches('_').is_empty() {
        "app".to_string()
    } else {
        safe_name.chars().take(40).collect()
    };

    // "v2" busts icons cached before shortcut targets were resolved. The v1
    // cache stored the shell shortcut icon, which includes the arrow overlay
    // in the bottom-left corner.
    let icon = dir.join(format!(
        "app_{}_{:08x}.png",
        safe_name,
        fnv_hash(&format!("v2:{}", path))
    ));
    if !icon.exists() {
        // Resolve .lnk shortcuts to their target executable first. Calling
        // ExtractAssociatedIcon directly on a shortcut returns the shell's
        // shortcut icon (with the arrow overlay) instead of the real app icon.
        let script = "& { param($p, $d) Add-Type -AssemblyName System.Drawing; $src = $p; if ($p -like '*.lnk') { $sh = New-Object -ComObject WScript.Shell; $sc = $sh.CreateShortcut($p); if ($sc.TargetPath -and (Test-Path $sc.TargetPath)) { $src = $sc.TargetPath } }; $i = [System.Drawing.Icon]::ExtractAssociatedIcon($src); if ($null -eq $i) { exit 1 }; $b = $i.ToBitmap(); $b.Save($d, [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose(); $i.Dispose() }";
        let icon_arg = icon.to_string_lossy().to_string();
        let output = StdCommand::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
                path.as_str(),
                icon_arg.as_str(),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Could not read application icon: {}", e))?;

        if !output.status.success() {
            logging::error(format!(
                "could not extract icon for {}: {}",
                path,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            return Ok(ApplicationInfo {
                path,
                name,
                icon_path: None,
            });
        }
    }

    Ok(ApplicationInfo {
        path,
        name,
        icon_path: Some(icon.to_string_lossy().to_string()),
    })
}

// --------------------------------------------------------------------------
// image search (editor helper)
// --------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct ImageSearchResult {
    title: String,
    url: String,
    thumbnail: String,
    source: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn search_steamgriddb(
    client: &reqwest::Client,
    query: &str,
    api_key: &str,
) -> Result<Vec<ImageSearchResult>, String> {
    let url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(query)
    );
    let resp = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("SteamGridDB returned {}", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let game = data["data"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("No game found on SteamGridDB")?;
    let game_id = game["id"].as_i64().ok_or("Invalid SteamGridDB response")?;
    let game_name = game["name"].as_str().unwrap_or(query).to_string();

    let icons_url = format!("https://www.steamgriddb.com/api/v2/icons/game/{}", game_id);
    let resp = client
        .get(&icons_url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("SteamGridDB icons returned {}", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for item in data["data"].as_array().into_iter().flatten().take(40) {
        let Some(url) = item["url"].as_str().filter(|u| !u.is_empty()) else {
            continue;
        };
        let thumbnail = item["thumb"].as_str().unwrap_or(url);
        results.push(ImageSearchResult {
            title: game_name.clone(),
            url: url.to_string(),
            thumbnail: thumbnail.to_string(),
            source: "SteamGridDB".to_string(),
        });
    }

    if results.is_empty() {
        return Err("No icons found on SteamGridDB".to_string());
    }
    Ok(results)
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn extract_quoted<'a>(haystack: &'a str, marker: &str) -> Option<&'a str> {
    let start = haystack.find(marker)? + marker.len();
    let end = haystack[start..].find("&quot;")? + start;
    Some(&haystack[start..end])
}

async fn search_bing(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<ImageSearchResult>, String> {
    let mut url = reqwest::Url::parse("https://www.bing.com/images/search")
        .map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("form", "HDRSC2")
        .append_pair("first", "1");

    let resp = client
        .get(url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Bing returned {}", resp.status()));
    }
    let html = resp.text().await.map_err(|e| e.to_string())?;

    const MURL: &str = "&quot;murl&quot;:&quot;";
    const TURL: &str = "&quot;turl&quot;:&quot;";
    const TITLE: &str = "&quot;t&quot;:&quot;";

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut index = 0;

    while let Some(pos) = html[index..].find(MURL) {
        let start = index + pos + MURL.len();
        let Some(relative_end) = html[start..].find("&quot;") else {
            break;
        };
        let murl = &html[start..start + relative_end];
        index = start + relative_end;

        if !murl.starts_with("http") || !seen.insert(murl.to_string()) {
            continue;
        }

        let window_end = (start + 4000).min(html.len());
        let window = &html[start..window_end];
        let thumbnail = extract_quoted(window, TURL).unwrap_or(murl);
        let title = extract_quoted(window, TITLE).unwrap_or("");

        results.push(ImageSearchResult {
            title: html_unescape(title),
            url: murl.to_string(),
            thumbnail: thumbnail.to_string(),
            source: "Bing".to_string(),
        });

        if results.len() >= 40 {
            break;
        }
    }

    if results.is_empty() {
        return Err("No images found".to_string());
    }
    Ok(results)
}

#[tauri::command]
async fn search_images(
    query: String,
    api_key: String,
    source: String,
) -> Result<Vec<ImageSearchResult>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Enter something to search for".to_string());
    }

    let client = http_client()?;
    let api_key = api_key.trim().to_string();

    match source.trim().to_ascii_lowercase().as_str() {
        "web" => search_bing(&client, &query).await,
        "steamgriddb" => {
            if api_key.is_empty() {
                return Err("Add a SteamGridDB API key to use this source".to_string());
            }
            search_steamgriddb(&client, &query, &api_key).await
        }
        _ => {
            if api_key.is_empty() {
                search_bing(&client, &query).await
            } else {
                match search_steamgriddb(&client, &query, &api_key).await {
                    Ok(results) => Ok(results),
                    Err(_) => search_bing(&client, &query).await,
                }
            }
        }
    }
}

fn normalize_image_extension(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "gif" => Some("gif"),
        "webp" => Some("webp"),
        "bmp" => Some("bmp"),
        "svg" => Some("svg"),
        "ico" => Some("ico"),
        "avif" => Some("avif"),
        _ => None,
    }
}

fn extension_from_url(url: &str) -> Option<&'static str> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let last = parsed.path().rsplit('/').next()?;
    let ext = last.rsplit_once('.')?.1;
    normalize_image_extension(ext)
}

fn extension_from_content_type(content_type: &str) -> Option<&'static str> {
    match content_type.split(';').next()?.trim() {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        "image/svg+xml" => Some("svg"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        "image/avif" => Some("avif"),
        _ => None,
    }
}

fn fnv_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[tauri::command]
async fn download_image(
    app: tauri::AppHandle,
    url: String,
    name: String,
) -> Result<String, String> {
    let client = http_client()?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Download failed: {}", resp.status()));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !content_type.is_empty() && !content_type.starts_with("image/") {
        return Err(format!("URL is not an image ({})", content_type));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("Downloaded image is empty".to_string());
    }

    let extension = extension_from_url(&url)
        .or_else(|| extension_from_content_type(&content_type))
        .unwrap_or("png");

    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("images");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe_name.truncate(40);
    if safe_name.trim_matches('_').is_empty() {
        safe_name = "image".to_string();
    }

    let file = dir.join(format!(
        "{}_{:08x}.{}",
        safe_name,
        fnv_hash(&url),
        extension
    ));
    fs::write(&file, &bytes).map_err(|e| e.to_string())?;

    Ok(file.to_string_lossy().to_string())
}

// --------------------------------------------------------------------------
// window / tray
// --------------------------------------------------------------------------

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())
    } else {
        Err("main window not found".to_string())
    }
}

#[tauri::command]
fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())
    } else {
        Err("main window not found".to_string())
    }
}

#[tauri::command]
fn quit_app(
    keyboard: tauri::State<'_, Arc<KeyboardManager>>,
    devices: tauri::State<'_, Arc<RawInputManager>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    devices.stop();
    keyboard.stop();
    logging::info("quit requested");
    app.exit(0);
    Ok(())
}

fn load_minimize_setting(app: &tauri::AppHandle) -> Option<bool> {
    let path = config_path(app);
    let content = fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(
        value
            .get("settings")
            .and_then(|s| s.get("general"))
            .and_then(|g| g.get("minimizeToTray"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    )
}

// --------------------------------------------------------------------------
// phone remote
// --------------------------------------------------------------------------

#[tauri::command]
fn get_remote_info(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<server::RemoteState>>,
) -> server::RemoteInfo {
    server::info(&app, state.inner())
}

#[tauri::command]
fn set_remote_enabled(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<server::RemoteState>>,
    enabled: bool,
) -> Result<server::RemoteInfo, String> {
    server::set_enabled(&app, state.inner(), enabled)?;
    Ok(server::info(&app, state.inner()))
}

#[tauri::command]
fn regenerate_pairing(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<server::RemoteState>>,
) -> server::RemoteInfo {
    server::regenerate(&app, state.inner())
}

// --------------------------------------------------------------------------
// bootstrap
// --------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let log_dir = app
                .path()
                .app_config_dir()
                .map(|dir| dir.join("logs"))
                .unwrap_or_else(|_| std::env::temp_dir());
            logging::init(log_dir);

            let core = AppCore::new(app.handle().clone());
            let (device_tx, device_rx) = channel::<DeviceKeyEvent>();
            let keyboard = KeyboardManager::new(core.clone(), device_rx, device_tx.clone());
            let devices = RawInputManager::new(app.handle().clone(), device_tx);
            RawInputManager::install(devices.clone());

            // Manage before starting: the device worker resolves the keyboard
            // manager through Tauri state when deciding whether macros are
            // paused.
            app.manage(core.clone());
            app.manage(keyboard.clone());
            app.manage(devices.clone());

            match keyboard.start() {
                Ok(()) => logging::info("native keyboard observer started"),
                Err(error) => logging::error(format!("keyboard observer failed: {}", error)),
            }
            match devices.start() {
                Ok(()) => logging::info("Raw Input keypad listener started"),
                Err(error) => logging::error(format!("Raw Input listener failed: {}", error)),
            }

            let remote_state = Arc::new(server::RemoteState::new());
            server::boot(app.handle(), &remote_state);

            tray::setup(app.handle(), core.clone(), keyboard.clone())?;

            app.manage(remote_state);

            logging::info("AYMAN MACRO DECK ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(devices) = window.app_handle().try_state::<Arc<RawInputManager>>() {
                    devices.stop();
                }
                if let Some(keyboard) = window.app_handle().try_state::<Arc<KeyboardManager>>() {
                    keyboard.stop();
                }
                return;
            }
            if let Some(config) = load_minimize_setting(window.app_handle()) {
                if !config {
                    return;
                }
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    tauri::WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                        if window.is_minimized().unwrap_or(false) {
                            let _ = window.hide();
                        }
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            reload_ahk_script,
            execute_action,
            trigger_button,
            get_app_state,
            get_gaming_mode,
            set_gaming_mode,
            get_keyboard_connected,
            set_shortcuts_paused,
            reload_shortcuts,
            get_keyboard_devices,
            refresh_keyboard_devices,
            get_selected_macro_device,
            select_macro_device,
            clear_macro_device_selection,
            start_device_test,
            stop_device_test,
            get_device_test_candidate,
            confirm_device_test,
            get_macro_engine_status,
            set_macro_engine_running,
            set_macro_device_collections,
            set_macro_device_suppression,
            get_start_with_windows,
            set_start_with_windows,
            open_logs_folder,
            export_config,
            import_config,
            list_config_backups,
            restore_config_backup,
            list_profiles,
            create_profile,
            rename_profile,
            duplicate_profile,
            delete_profile,
            set_active_profile,
            list_audio_devices,
            list_lnk_files,
            get_application_info,
            search_images,
            download_image,
            hide_window,
            show_window,
            quit_app,
            get_remote_info,
            set_remote_enabled,
            regenerate_pairing
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
