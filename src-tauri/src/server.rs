//! LAN HTTP API used by the phone app.
//!
//! Runs on the home network only and requires a pairing code + token, so the
//! phone can read the deck and press buttons over WiFi.

use std::fs;
use std::io::Read;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::keyboard::KeyboardManager;
use crate::state::AppCore;

pub const DEFAULT_PORT: u16 = 8787;
const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSettings {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
    pub pair_code: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: DEFAULT_PORT,
            token: String::new(),
            pair_code: String::new(),
        }
    }
}

pub struct RemoteState {
    settings: Mutex<RemoteSettings>,
    server: Mutex<Option<Arc<Server>>>,
    error: Mutex<String>,
}

impl RemoteState {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(RemoteSettings::default()),
            server: Mutex::new(None),
            error: Mutex::new(String::new()),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct RemoteInfo {
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    pub pair_code: String,
    pub addresses: Vec<String>,
    pub error: String,
}

fn settings_file(app: &tauri::AppHandle) -> PathBuf {
    crate::config_path(app)
        .parent()
        .map(|dir| dir.join("remote.json"))
        .unwrap_or_else(|| PathBuf::from("remote.json"))
}

fn random_hex(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let byte: u8 = rng.gen_range(0..16);
            format!("{:x}", byte)
        })
        .collect()
}

fn random_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| format!("{}", rng.gen_range(0..10)))
        .collect()
}

fn token() -> String {
    random_hex(32)
}

fn load(app: &tauri::AppHandle) -> RemoteSettings {
    let path = settings_file(app);
    let mut settings = fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<RemoteSettings>(&content).ok())
        .unwrap_or_default();

    if settings.port == 0 {
        settings.port = DEFAULT_PORT;
    }
    if settings.token.trim().is_empty() {
        settings.token = token();
    }
    if settings.pair_code.trim().len() != 6 {
        settings.pair_code = random_code();
    }
    settings
}

fn save(app: &tauri::AppHandle, settings: &RemoteSettings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let path = settings_file(app);
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, json);
    }
}

/// Local IPv4 addresses of this computer, as seen from the home network.
fn local_addresses() -> Vec<String> {
    let mut result = Vec::new();

    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" {
                    result.push(ip);
                }
            }
        }
    }

    if result.is_empty() {
        if let Ok(output) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' }).IPAddress",
            ])
            .output()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let ip = line.trim();
                if !ip.is_empty() && !result.iter().any(|r| r == ip) {
                    result.push(ip.to_string());
                }
            }
        }
    }

    result
}

pub fn info(_app: &tauri::AppHandle, state: &RemoteState) -> RemoteInfo {
    let settings = state.settings.lock().unwrap().clone();
    RemoteInfo {
        enabled: settings.enabled,
        running: state.server.lock().unwrap().is_some(),
        port: settings.port,
        pair_code: settings.pair_code,
        addresses: local_addresses(),
        error: state.error.lock().unwrap().clone(),
    }
}

pub fn start(app: &tauri::AppHandle, state: Arc<RemoteState>) -> Result<(), String> {
    let mut guard = state.server.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }

    let settings = state.settings.lock().unwrap().clone();
    let server = Server::http(("0.0.0.0", settings.port))
        .map_err(|e| format!("cannot listen on port {}: {}", settings.port, e))?;
    let server = Arc::new(server);
    *guard = Some(server.clone());
    *state.error.lock().unwrap() = String::new();
    drop(guard);

    let app_handle = (*app).clone();
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let app = app_handle.clone();
            let state = state.clone();
            std::thread::spawn(move || handle(request, app, state));
        }
    });

    Ok(())
}

pub fn stop(state: &RemoteState) {
    if let Some(server) = state.server.lock().unwrap().take() {
        server.unblock();
    }
}

pub fn set_enabled(app: &tauri::AppHandle, state: &Arc<RemoteState>, enabled: bool) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().unwrap();
        settings.enabled = enabled;
        save(app, &settings);
    }
    if enabled {
        start(app, state.clone())
    } else {
        stop(state);
        Ok(())
    }
}

pub fn regenerate(app: &tauri::AppHandle, state: &RemoteState) -> RemoteInfo {
    {
        let mut settings = state.settings.lock().unwrap();
        settings.pair_code = random_code();
        settings.token = token();
        save(app, &settings);
    }
    info(app, state)
}

pub fn boot(app: &tauri::AppHandle, state: &Arc<RemoteState>) {
    let settings = load(app);
    let enabled = settings.enabled;
    *state.settings.lock().unwrap() = settings.clone();
    save(app, &settings);

    if enabled {
        if let Err(err) = start(app, state.clone()) {
            *state.error.lock().unwrap() = err;
        }
    }
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
}

fn cors_origin() -> Header {
    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
}

fn reply_json(request: tiny_http::Request, status: u16, body: serde_json::Value) {
    let payload = body.to_string();
    let response = Response::from_string(payload)
        .with_status_code(StatusCode(status))
        .with_header(json_header())
        .with_header(cors_origin());
    let _ = request.respond(response);
}

fn reply_error(request: tiny_http::Request, status: u16, message: &str) {
    reply_json(request, status, serde_json::json!({ "error": message }));
}

fn read_body(request: &mut tiny_http::Request) -> serde_json::Value {
    let mut buffer = String::new();
    let _ = request
        .as_reader()
        .take(MAX_BODY as u64)
        .read_to_string(&mut buffer);
    serde_json::from_str(&buffer).unwrap_or(serde_json::Value::Null)
}

fn header_value<'a>(request: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

fn authorized(request: &tiny_http::Request, state: &RemoteState) -> bool {
    let Some(token) = header_value(request, "X-Deck-Token") else {
        return false;
    };
    let settings = state.settings.lock().unwrap();
    !settings.token.is_empty() && token.trim() == settings.token
}

fn handle(mut request: tiny_http::Request, app: tauri::AppHandle, state: Arc<RemoteState>) {
    let method = request.method().clone();
    let full_url = request.url().to_string();
    let path = full_url
        .split('?')
        .next()
        .unwrap_or("/")
        .trim_end_matches('/')
        .to_string();
    let path = if path.is_empty() { "/".to_string() } else { path };

    if method == Method::Options {
        let _ = request.respond(
            Response::empty(StatusCode(204))
                .with_header(cors_origin())
                .with_header(
                    Header::from_bytes(
                        &b"Access-Control-Allow-Headers"[..],
                        &b"Content-Type, X-Deck-Token"[..],
                    )
                    .unwrap(),
                )
                .with_header(
                    Header::from_bytes(
                        &b"Access-Control-Allow-Methods"[..],
                        &b"GET, POST, OPTIONS"[..],
                    )
                    .unwrap(),
                ),
        );
        return;
    }

    match (&method, path.as_str()) {
        (Method::Get, "/api/ping") => {
            reply_json(
                request,
                200,
                serde_json::json!({
                    "ok": true,
                    "app": "AYMAN MACRO DECK",
                    "version": env!("CARGO_PKG_VERSION"),
                    "intent": "phone-remote",
                }),
            );
        }

        (Method::Post, "/api/pair") => {
            let body = read_body(&mut request);
            let code = body
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let settings = state.settings.lock().unwrap().clone();
            if code.is_empty() || code != settings.pair_code {
                reply_error(request, 401, "Wrong pairing code");
                return;
            }
            reply_json(
                request,
                200,
                serde_json::json!({
                    "token": settings.token,
                    "app": "AYMAN MACRO DECK",
                    "version": env!("CARGO_PKG_VERSION"),
                }),
            );
        }

        (Method::Get, "/api/state") => {
            if !authorized(&request, &state) {
                reply_error(request, 401, "Not paired");
                return;
            }
            match deck_state(&app) {
                Ok(value) => reply_json(request, 200, value),
                Err(err) => reply_error(request, 500, &err),
            }
        }

        (Method::Post, "/api/press") => {
            if !authorized(&request, &state) {
                reply_error(request, 401, "Not paired");
                return;
            }
            let body = read_body(&mut request);
            match press_button(&app, &body) {
                Ok(value) => reply_json(request, 200, value),
                Err((status, message)) => reply_error(request, status, &message),
            }
        }

        _ => reply_error(request, 404, "Not found"),
    }
}

/// The live config comes from the state manager, not from disk, so the phone
/// always sees exactly what the keyboard engine is acting on.
fn read_config(app: &tauri::AppHandle) -> serde_json::Value {
    match core_of(app) {
        Some(core) => serde_json::to_value(core.config()).unwrap_or(serde_json::Value::Null),
        None => serde_json::Value::Null,
    }
}

fn core_of(app: &tauri::AppHandle) -> Option<std::sync::Arc<AppCore>> {
    use tauri::Manager;
    app.try_state::<std::sync::Arc<AppCore>>()
        .map(|state| state.inner().clone())
}

fn current_gaming(app: &tauri::AppHandle) -> bool {
    core_of(app).map(|core| core.gaming()).unwrap_or(false)
}

/// Mirrors the engine's profile choice: ScrollLock picks Gaming/Normal, with the
/// app's active profile as a fallback.
fn active_profile(config: &serde_json::Value, mode_is_gaming: bool) -> Option<serde_json::Value> {
    let profiles = config.get("profiles")?.as_array()?;
    let wanted = if mode_is_gaming { "gaming" } else { "normal" };

    if let Some(found) = profiles.iter().find(|p| {
        p.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.eq_ignore_ascii_case(wanted))
            .unwrap_or(false)
    }) {
        return Some(found.clone());
    }

    let active_id = config
        .get("activeProfileId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    profiles
        .iter()
        .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(active_id))
        .or_else(|| profiles.first())
        .cloned()
}

fn deck_state(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = read_config(app);
    if config.is_null() {
        return Err("Config not found".to_string());
    }

    let gaming = current_gaming(app);
    let profile = active_profile(&config, gaming)
        .ok_or_else(|| "No profile configured".to_string())?;

    let mut buttons: Vec<serde_json::Value> = profile
        .get("buttons")
        .and_then(|b| b.as_array())
        .map(|list| {
            list.iter()
                .map(|b| {
                    serde_json::json!({
                        "id": b.get("id").cloned().unwrap_or(serde_json::Value::Null),
                        "key": b.get("key").cloned().unwrap_or(serde_json::Value::Null),
                        "name": b.get("name").cloned().unwrap_or(serde_json::Value::Null),
                        "icon": b.get("icon").cloned().unwrap_or(serde_json::Value::Null),
                        "description": b.get("description").cloned().unwrap_or(serde_json::Value::Null),
                        "enabled": b.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                        "actionCount": b.get("actions").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    buttons.sort_by_key(|b| b.get("key").and_then(|k| k.as_i64()).unwrap_or(99));

    let profiles: Vec<serde_json::Value> = config
        .get("profiles")
        .and_then(|p| p.as_array())
        .map(|list| {
            list.iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.get("id").cloned().unwrap_or(serde_json::Value::Null),
                        "name": p.get("name").cloned().unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(serde_json::json!({
        "app": "AYMAN MACRO DECK",
        "version": env!("CARGO_PKG_VERSION"),
        "gamingMode": gaming,
        "mode": if gaming { "Gaming" } else { "Normal" },
        "profileId": profile.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "profileName": profile.get("name").cloned().unwrap_or(serde_json::Value::Null),
        "profiles": profiles,
        "buttons": buttons,
        "buttonCount": buttons.len(),
    }))
}

fn press_button(
    app: &tauri::AppHandle,
    body: &serde_json::Value,
) -> Result<serde_json::Value, (u16, String)> {
    let config = read_config(app);
    if config.is_null() {
        return Err((500, "Config not found".to_string()));
    }

    let gaming = current_gaming(app);
    let profile = active_profile(&config, gaming)
        .ok_or((500, "No profile configured".to_string()))?;

    let body_id = body.get("buttonId").and_then(|v| v.as_str());
    let body_key = body.get("key").and_then(|v| v.as_i64());

    let button = profile
        .get("buttons")
        .and_then(|b| b.as_array())
        .and_then(|list| {
            list.iter().find(|b| {
                if let Some(id) = body_id {
                    b.get("id").and_then(|i| i.as_str()) == Some(id)
                } else if let Some(key) = body_key {
                    b.get("key").and_then(|k| k.as_i64()) == Some(key)
                } else {
                    false
                }
            })
        })
        .cloned()
        .ok_or((404, "Button not found in the active profile".to_string()))?;

    if !button.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true) {
        return Err((409, "Button is disabled".to_string()));
    }

    let name = button
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Button")
        .to_string();

    let results = run_button(app, &button, &name);

    Ok(serde_json::json!({
        "ok": true,
        "button": name,
        "results": results,
        "state": deck_state(app).unwrap_or(serde_json::Value::Null),
    }))
}

/// Phone presses go through the same dispatcher as physical keys.
fn run_button(app: &tauri::AppHandle, button: &serde_json::Value, name: &str) -> Vec<String> {
    use tauri::Manager;

    let button_id = button
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or("")
        .to_string();

    if let Some(keyboard) = app.try_state::<std::sync::Arc<KeyboardManager>>() {
        crate::logging::info(format!("phone press -> {} ({})", name, button_id));
        return keyboard.dispatch_button(&button_id);
    }

    match core_of(app).and_then(|core| core.find_button(&button_id)) {
        Some(button) => match core_of(app) {
            Some(core) => crate::actions::run_button(&core, &button),
            None => vec!["state is not ready".to_string()],
        },
        None => vec![format!("button {} not found", button_id)],
    }
}


