use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tauri::Manager;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYBDINPUT, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn config_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("failed to get app config dir");
    fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<String, String> {
    let path = config_path(&app);
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok(String::from("{}"))
    }
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: String) -> Result<(), String> {
    let path = config_path(&app);
    fs::write(&path, config).map_err(|e| e.to_string())
}

#[tauri::command]
fn reload_ahk_script(ahk_executable: String, script_path: String) -> Result<(), String> {
    let executable = if ahk_executable.trim().is_empty() {
        "AutoHotkey.exe"
    } else {
        ahk_executable.as_str()
    };

    StdCommand::new(executable)
        .arg(script_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn execute_action(action_type: String, params: serde_json::Value) -> Result<String, String> {
    match action_type.as_str() {
        "open_application" => {
            let path = params["path"]
                .as_str()
                .ok_or("missing 'path' parameter")?;
            open::that(path).map_err(|e| e.to_string())?;
            Ok(format!("Opened: {}", path))
        }

        "open_url" => {
            let url = params["url"]
                .as_str()
                .ok_or("missing 'url' parameter")?;
            open::that(url).map_err(|e| e.to_string())?;
            Ok(format!("Opened URL: {}", url))
        }

        "switch_steam_account" => {
            let account = params["account"]
                .as_str()
                .ok_or("missing 'account' parameter")?;
            if let Some(shortcuts) = params["shortcuts"].as_object() {
                if let Some(path_val) = shortcuts.get(account) {
                    let path = path_val.as_str().ok_or("invalid shortcut path")?;
                    open::that(path).map_err(|e| e.to_string())?;
                    return Ok(format!("Switched Steam account: {}", account));
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
            Ok(format!("Launched Steam game: {}", app_id))
        }

        "launch_epic_game" => {
            let uri = params["uri"]
                .as_str()
                .ok_or("missing 'uri' parameter")?;
            open::that(uri).map_err(|e| e.to_string())?;
            Ok("Launched Epic game".to_string())
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
            Ok(format!("Audio switched to: {}", device))
        }

        "toggle_gaming_mode" => {
            use windows::Win32::UI::Input::KeyboardAndMouse::VK_SCROLL;
            unsafe {
                let state = GetKeyState(VK_SCROLL.0 as i32);
                let is_on = (state & 1) != 0;
                let action = if is_on {
                    KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_EXTENDEDKEY
                };
                let input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_SCROLL,
                            dwFlags: action,
                            ..Default::default()
                        },
                    },
                };
                let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                let new_state = !is_on;
                Ok(serde_json::json!({
                    "message": if new_state { "Gaming Mode ON" } else { "Gaming Mode OFF" },
                    "gamingMode": new_state
                })
                .to_string())
            }
        }

        "open_folder" => {
            let path = params["path"]
                .as_str()
                .ok_or("missing 'path' parameter")?;
            open::that(path).map_err(|e| e.to_string())?;
            Ok(format!("Opened folder: {}", path))
        }

        "run_command" => {
            let command = params["command"]
                .as_str()
                .ok_or("missing 'command' parameter")?;
            let args: Vec<String> = match &params["args"] {
                serde_json::Value::Array(a) => a
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                serde_json::Value::String(s) => s
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
            Ok(format!("Ran command: {}", command))
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
            Ok(format!("Ran AHK script: {}", script_path))
        }

        "keyboard_shortcut" => {
            let keys_str = params["keys"]
                .as_str()
                .ok_or("missing 'keys' parameter")?;
            let key_names: Vec<String> = keys_str
                .split('+')
                .map(|k| k.trim().to_lowercase())
                .filter(|k| !k.is_empty())
                .collect();
            if key_names.is_empty() {
                return Err("no keys specified".to_string());
            }

            let virtual_keys: Vec<u16> = key_names
                .iter()
                .map(|k| key_name_to_vk(k))
                .collect::<Option<Vec<_>>>()
                .ok_or("unknown key name in shortcut")?;

            let mut inputs: Vec<INPUT> = Vec::new();
            for &vk in &virtual_keys {
                inputs.push(make_key_input(vk, false));
            }
            for &vk in virtual_keys.iter().rev() {
                inputs.push(make_key_input(vk, true));
            }

            unsafe {
                let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
            Ok(format!("Sent shortcut: {}", keys_str))
        }

        "volume_up" => {
            send_media_key(VK_VOLUME_UP);
            Ok("Volume up".to_string())
        }

        "volume_down" => {
            send_media_key(VK_VOLUME_DOWN);
            Ok("Volume down".to_string())
        }

        "mute" => {
            send_media_key(VK_VOLUME_MUTE);
            Ok("Mute toggled".to_string())
        }

        "rdr2_cycle" => {
            // RDR2 cycle (backup saves -> uninstall Rockstar Launcher ->
            // silent fresh install) is orchestrated by the AHK engine
            // (RunRdr2Cycle in steam_mode.ahk).
            Ok("RDR2 cycle handled by AHK engine".to_string())
        }

        _ => Err(format!("Unknown action type: {}", action_type)),
    }
}

fn send_media_key(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
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

fn make_key_input(vk: u16, key_up: bool) -> INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_EXTENDEDKEY
    };
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
    match name {
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
        "f1" => Some(0x70),
        "f2" => Some(0x71),
        "f3" => Some(0x72),
        "f4" => Some(0x73),
        "f5" => Some(0x74),
        "f6" => Some(0x75),
        "f7" => Some(0x76),
        "f8" => Some(0x77),
        "f9" => Some(0x78),
        "f10" => Some(0x79),
        "f11" => Some(0x7A),
        "f12" => Some(0x7B),
        "a" => Some(0x41),
        "b" => Some(0x42),
        "c" => Some(0x43),
        "d" => Some(0x44),
        "e" => Some(0x45),
        "f" => Some(0x46),
        "g" => Some(0x47),
        "h" => Some(0x48),
        "i" => Some(0x49),
        "j" => Some(0x4A),
        "k" => Some(0x4B),
        "l" => Some(0x4C),
        "m" => Some(0x4D),
        "n" => Some(0x4E),
        "o" => Some(0x4F),
        "p" => Some(0x50),
        "q" => Some(0x51),
        "r" => Some(0x52),
        "s" => Some(0x53),
        "t" => Some(0x54),
        "u" => Some(0x55),
        "v" => Some(0x56),
        "w" => Some(0x57),
        "x" => Some(0x58),
        "y" => Some(0x59),
        "z" => Some(0x5A),
        "0" => Some(0x30),
        "1" => Some(0x31),
        "2" => Some(0x32),
        "3" => Some(0x33),
        "4" => Some(0x34),
        "5" => Some(0x35),
        "6" => Some(0x36),
        "7" => Some(0x37),
        "8" => Some(0x38),
        "9" => Some(0x39),
        "printscreen" | "prtsc" => Some(0x2C),
        "capslock" => Some(0x14),
        "numlock" => Some(0x90),
        "scrolllock" => Some(0x91),
        _ => None,
    }
}

#[tauri::command]
fn get_gaming_mode() -> Result<bool, String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_SCROLL;
    unsafe {
        let state = GetKeyState(VK_SCROLL.0 as i32);
        Ok((state & 1) != 0)
    }
}

#[tauri::command]
fn get_keyboard_connected() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
fn list_audio_devices(sound_volume_view_path: String) -> Result<Vec<String>, String> {
    let svv_path = if sound_volume_view_path.trim().is_empty() {
        "SoundVolumeView.exe".to_string()
    } else {
        sound_volume_view_path
    };

    let out_file = std::env::temp_dir().join(format!(
        "svv_{}.csv",
        std::process::id()
    ));

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

    // Strip UTF-8 BOM if present
    if content.starts_with('\u{FEFF}') {
        content = content.trim_start_matches('\u{FEFF}').to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return Ok(fallback_audio_devices());
    }

    // Parse CSV respecting quotes
    let parse_csv_line = |line: &str| -> Vec<String> {
        let mut fields = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    fields.push(cur.trim().to_string());
                    cur = String::new();
                }
                _ => cur.push(c),
            }
        }
        fields.push(cur.trim().to_string());
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
        let Some(rel_end) = html[start..].find("&quot;") else {
            break;
        };
        let murl = &html[start..start + rel_end];
        index = start + rel_end;

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
    let query = query.trim();
    if query.is_empty() {
        return Err("Enter something to search for".to_string());
    }

    let client = http_client()?;
    let api_key = api_key.trim();

    match source.trim().to_ascii_lowercase().as_str() {
        "web" => search_bing(&client, query).await,
        "steamgriddb" => {
            if api_key.is_empty() {
                return Err("Add a SteamGridDB API key to use this source".to_string());
            }
            search_steamgriddb(&client, query, api_key).await
        }
        _ => {
            if api_key.is_empty() {
                return search_bing(&client, query).await;
            }
            match search_steamgriddb(&client, query, api_key).await {
                Ok(results) => Ok(results),
                Err(_) => search_bing(&client, query).await,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
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
            get_gaming_mode,
            get_keyboard_connected,
            list_audio_devices,
            list_lnk_files,
            search_images,
            download_image,
            hide_window,
            show_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    // Force close all windows to exit the process
    app.exit(0);
    Ok(())
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("window icon".into()))?;

    let show_i = tauri::menu::MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_i = tauri::menu::MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = tauri::menu::Menu::with_items(
        app,
        &[&show_i, &hide_i, &tauri::menu::PredefinedMenuItem::separator(app)?, &quit_i],
    )?;

    let _tray = tauri::tray::TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("AYMAN MACRO DECK")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
