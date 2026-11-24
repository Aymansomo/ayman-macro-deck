//! StateManager — the single source of truth for the running backend.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::keyboard::KeyboardStatus;
use crate::models::{AppConfig, MacroButton, Settings};

pub struct AppCore {
    app: AppHandle,
    config: RwLock<AppConfig>,
    gaming: AtomicBool,
    /// Open deck page (folder) per profile id; absent means the main page.
    /// Also written to `session.json`, so an open page survives a restart.
    pages: Mutex<HashMap<String, String>>,
    /// Toggle-button states, kept for the lifetime of the app.
    toggles: Mutex<HashMap<String, bool>>,
    config_error: Mutex<Option<String>>,
    started: Instant,
}

/// Open pages out of a session document (`{ "openPages": { profile: page } }`).
pub(crate) fn pages_from_session(value: &Value) -> HashMap<String, String> {
    let mut pages = HashMap::new();
    if let Some(map) = value.get("openPages").and_then(|item| item.as_object()) {
        for (profile_id, page_id) in map {
            if let Some(id) = page_id.as_str() {
                pages.insert(profile_id.clone(), id.to_string());
            }
        }
    }
    pages
}

/// Open deck pages as stored on disk: profile id -> page id.
fn load_pages(path: &Path) -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        crate::logging::warn("session file is unreadable, starting on main pages");
        return HashMap::new();
    };
    pages_from_session(&value)
}

fn write_pages(path: &Path, pages: &HashMap<String, String>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&json!({ "openPages": pages }))
        .map_err(|error| format!("cannot serialize session: {}", error))?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create session dir: {}", e))?;
    }
    fs::write(path, text).map_err(|e| format!("cannot write session: {}", e))
}

/// Writes a timestamped backup of the original config before saving a migrated
/// copy. The backup uses the raw original text, so even unknown JSON fields that
/// the typed model does not understand are preserved byte-for-byte.
fn backup_and_write(path: &Path, original: &str, config: &AppConfig) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let backup = path.with_extension(format!("json.bak-{}", stamp));
    if let Some(dir) = backup.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create backup dir: {}", e))?;
    }
    fs::write(&backup, original).map_err(|e| format!("cannot write config backup: {}", e))?;

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create config dir: {}", e))?;
    }
    let migrated = config
        .to_json()
        .map_err(|e| format!("cannot serialize migrated config: {}", e))?;
    fs::write(path, migrated).map_err(|e| format!("cannot write migrated config: {}", e))?;
    Ok(backup)
}

impl AppCore {
    pub fn new(app: AppHandle) -> Arc<Self> {
        let path = crate::config_path(&app);
        let original_text = fs::read_to_string(&path).ok();
        let mut config = match original_text.as_deref() {
            Some(text) => match AppConfig::from_json(text) {
                Ok(config) => config,
                Err(error) => {
                    crate::logging::error(format!("cannot read config: {}", error));
                    AppConfig::default()
                }
            },
            None => {
                crate::logging::info("no config file yet, starting with an empty config");
                AppConfig::default()
            }
        };

        // Stage 3 migration is additive. Back up the exact original bytes before
        // the first write so no assignment, action or unknown field can be lost.
        if config.migrate_layouts() {
            if let Some(original) = original_text.as_deref() {
                match backup_and_write(&path, original, &config) {
                    Ok(backup) => crate::logging::info(format!(
                        "config layout migration applied; backup: {}",
                        backup.display()
                    )),
                    Err(error) => crate::logging::error(format!(
                        "config migration could not be persisted: {}",
                        error
                    )),
                }
            }
        }

        let config_error = config.validate().err();
        if let Some(error) = &config_error {
            crate::logging::warn(format!("config has problems: {}", error));
        }

        let gaming = false;
        crate::logging::info(format!(
            "state loaded: {} profile(s), active={}",
            config.profiles.len(),
            config.active_profile_id
        ));

        let session_path = crate::ui_state_path(&app);
        let pages = load_pages(&session_path);
        if !pages.is_empty() {
            crate::logging::info(format!(
                "session loaded: {} open page(s) restored",
                pages.len()
            ));
        }

        Arc::new(Self {
            app,
            config: RwLock::new(config),
            gaming: AtomicBool::new(gaming),
            pages: Mutex::new(pages),
            toggles: Mutex::new(HashMap::new()),
            config_error: Mutex::new(config_error),
            started: Instant::now(),
        })
    }

    pub fn app(&self) -> &AppHandle {
        &self.app
    }

    pub fn config(&self) -> AppConfig {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn settings(&self) -> Settings {
        self.config.read().expect("config lock poisoned").settings.clone()
    }

    pub fn gaming(&self) -> bool {
        self.gaming.load(Ordering::SeqCst)
    }

    pub fn set_gaming(&self, gaming: bool) {
        if self.gaming.swap(gaming, Ordering::SeqCst) != gaming {
            crate::logging::info(format!(
                "mode -> {}",
                if gaming { "Gaming" } else { "Normal" }
            ));
        }
    }

    /// Activates a profile by id or name. Buttons created in the editor store
    /// the profile id (stable across renames); older configs store the name, so
    /// both are accepted.
    pub fn apply_profile_switch(&self, profile: &str) {
        let resolved = {
            let config = self.config.read().expect("config lock poisoned");
            config
                .profile(profile)
                .or_else(|| config.profile_by_name(profile))
                .map(|p| (p.id.clone(), p.name.clone(), p.apps.clone()))
        };

        let Some((id, name, apps)) = resolved else {
            crate::logging::warn(format!("profile switch target not found: {}", profile));
            return;
        };

        self.set_gaming(name.eq_ignore_ascii_case("Gaming"));

        // The page each profile had open is remembered per profile, so switching
        // away and back returns to it instead of resetting to the main page.

        {
            let mut config = self.config.write().expect("config lock poisoned");
            config.active_profile_id = id;
        }

        // Apps configured on the profile start (or move) in the background.
        crate::apps::apply(&apps);
    }

    /// Open deck page for a profile, if any. A stored page that no longer
    /// exists (deleted, or an older config) is ignored, so the deck falls back
    /// to the main page safely.
    pub fn page_for(&self, profile_id: &str) -> Option<String> {
        let page = self
            .pages
            .lock()
            .expect("page lock poisoned")
            .get(profile_id)
            .cloned()?;
        let config = self.config.read().expect("config lock poisoned");
        let profile = config.profile(profile_id)?;
        profile
            .pages
            .iter()
            .any(|candidate| candidate.id == page)
            .then_some(page)
    }

    /// Saves the open pages, so the deck reopens where the user left it.
    fn persist_pages(&self, pages: &HashMap<String, String>) {
        let path = crate::ui_state_path(&self.app);
        if let Err(error) = write_pages(&path, pages) {
            crate::logging::warn(format!("cannot save open pages: {}", error));
        }
    }

    /// Switches the bound profile to a page (folder). An empty target, `root`,
    /// `main` or `back` returns to the profile's main page. Returns the page
    /// name that is now open, or `None` when the target does not exist.
    pub fn open_page(&self, target: &str) -> Option<String> {
        let (profile_id, page) = {
            let config = self.config.read().expect("config lock poisoned");
            let profile = config.active_profile(self.gaming())?.clone();
            let trimmed = target.trim();
            let is_root = trimmed.is_empty()
                || matches!(
                    trimmed.to_ascii_lowercase().as_str(),
                    "root" | "main" | "back"
                );
            let page = if is_root {
                (None, "Main".to_string())
            } else {
                let found = profile
                    .page(trimmed)
                    .or_else(|| profile.page_by_name(trimmed))?;
                (Some(found.id.clone()), found.name.clone())
            };
            (profile.id.clone(), page)
        };

        let mut pages = self.pages.lock().expect("page lock poisoned");
        match page.0 {
            Some(id) => {
                pages.insert(profile_id, id);
            }
            None => {
                pages.remove(&profile_id);
            }
        }
        let snapshot = pages.clone();
        drop(pages);
        self.persist_pages(&snapshot);
        Some(page.1)
    }

    /// Replaces the open pages, e.g. after a restore, and applies them without
    /// a restart: the engine keeps its own copy, so writing the session file
    /// alone would only show up after the next start. Pages that do not exist
    /// in the current config are dropped. Returns how many were applied.
    pub fn replace_pages(&self, pages: HashMap<String, String>) -> usize {
        let kept: HashMap<String, String> = {
            let config = self.config.read().expect("config lock poisoned");
            pages
                .into_iter()
                .filter(|(profile_id, page_id)| {
                    config
                        .profile(profile_id)
                        .map(|profile| profile.pages.iter().any(|page| &page.id == page_id))
                        .unwrap_or(false)
                })
                .collect()
        };
        let count = kept.len();
        {
            let mut current = self.pages.lock().expect("page lock poisoned");
            *current = kept.clone();
        }
        self.persist_pages(&kept);
        count
    }

    /// Flips a toggle button and returns its new state.
    pub fn toggle_state(&self, button_id: &str) -> bool {
        let mut toggles = self.toggles.lock().expect("toggle lock poisoned");
        let entry = toggles.entry(button_id.to_string()).or_insert(false);
        *entry = !*entry;
        *entry
    }

    pub fn toggle_values(&self) -> HashMap<String, bool> {
        self.toggles.lock().expect("toggle lock poisoned").clone()
    }

    pub fn find_button(&self, button_id: &str) -> Option<MacroButton> {
        let config = self.config.read().expect("config lock poisoned");
        for profile in &config.profiles {
            for button in profile.all_buttons() {
                if button.id == button_id {
                    return Some(button.clone());
                }
            }
        }
        None
    }

    /// Replaces the in-memory config from the JSON the editor sends and writes
    /// it to disk (raw text kept verbatim so nothing is lost).
    pub fn replace_config_json(&self, json_text: &str) -> Result<(), String> {
        let parsed = AppConfig::from_json(json_text)?;
        let validation = parsed.validate().err();
        *self.config.write().expect("config lock poisoned") = parsed;
        *self.config_error.lock().expect("config error lock poisoned") = validation.clone();

        if let Some(error) = &validation {
            crate::logging::warn(format!("config validation: {}", error));
        }

        self.persist()
    }

    pub fn set_config(&self, config: AppConfig) -> Result<(), String> {
        let validation = config.validate().err();
        let text = config.to_json()?;
        *self.config.write().expect("config lock poisoned") = config;
        *self.config_error.lock().expect("config error lock poisoned") = validation;
        self.write_config_text(&text)
    }

    /// Writes a timestamped backup of the current config without modifying it.
    pub fn backup_current_config(&self) -> Result<PathBuf, String> {
        let path = crate::config_path(&self.app);
        let original = fs::read_to_string(&path).unwrap_or_default();
        let config = self.config.read().expect("config lock poisoned").clone();
        backup_and_write(&path, &original, &config)
    }

    pub fn persist(&self) -> Result<(), String> {
        let text = self.config.read().expect("config lock poisoned").to_json()?;
        self.write_config_text(&text)
    }

    fn write_config_text(&self, text: &str) -> Result<(), String> {
        let path = crate::config_path(&self.app);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("cannot create config dir: {}", e))?;
        }
        fs::write(&path, text).map_err(|e| format!("cannot write config: {}", e))?;
        // Hotkeys may have changed with the saved config.
        crate::hotkeys::notify_changed();
        Ok(())
    }

    pub fn config_json(&self) -> String {
        self.config
            .read()
            .expect("config lock poisoned")
            .to_json()
            .unwrap_or_else(|_| "{}".to_string())
    }

    /// Automatic profile switching is on unless the user turned it off.
    pub fn auto_switch_enabled(&self) -> bool {
        self.config
            .read()
            .expect("config lock poisoned")
            .settings
            .extra
            .get("general")
            .and_then(|general| general.get("autoSwitchProfiles"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
    }

    /// True when at least one profile is linked to an app. Read without cloning
    /// the config so the foreground watcher can check it on every tick.
    pub fn has_linked_profiles(&self) -> bool {
        self.config
            .read()
            .expect("config lock poisoned")
            .profiles
            .iter()
            .any(|profile| {
                profile
                    .linked_app
                    .as_deref()
                    .map(|app| !app.trim().is_empty())
                    .unwrap_or(false)
            })
    }

    pub fn state_json(&self, keyboard: &KeyboardStatus) -> Value {
        let config = self.config.read().expect("config lock poisoned");
        let bound = config.active_profile(self.gaming());
        let active = config
            .profile(&config.active_profile_id)
            .or_else(|| config.profiles.first());

        let profiles: Vec<Value> = config
            .profiles
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.name,
                    "buttonCount": p.buttons.len(),
                })
            })
            .collect();

        let pages: Vec<Value> = bound
            .map(|p| {
                p.pages
                    .iter()
                    .map(|page| json!({ "id": page.id, "name": page.name }))
                    .collect()
            })
            .unwrap_or_default();
        let active_page = bound.and_then(|p| self.page_for(&p.id));
        let toggle_states = self.toggle_values();

        json!({
            "app": "AYMAN MACRO DECK",
            "version": env!("CARGO_PKG_VERSION"),
            "configVersion": config.version,
            "activeProfileId": config.active_profile_id,
            "activeProfileName": active.map(|p| p.name.clone()),
            "boundProfileName": bound.map(|p| p.name.clone()),
            "profiles": profiles,
            "gamingMode": self.gaming(),
            "mode": if self.gaming() { "Gaming" } else { "Normal" },
            "pages": pages,
            "activePageId": active_page,
            "toggleStates": toggle_states,
            "paused": keyboard.paused,
            "keyboardRunning": keyboard.running,
            "keyboardError": keyboard.error,
            "shortcuts": keyboard.shortcuts,
            "shortcutCount": keyboard.shortcut_count,
            "eventsSeen": keyboard.events_seen,
            "triggers": keyboard.triggers,
            "configError": self.config_error.lock().expect("config error lock poisoned").clone(),
            "uptimeSeconds": self.started.elapsed().as_secs(),
        })
    }

    pub fn emit_state(&self, keyboard: &KeyboardStatus) {
        let payload = self.state_json(keyboard);
        if let Err(error) = self.app.emit("deck:state", payload) {
            crate::logging::warn(format!("cannot emit state: {}", error));
        }
    }
}

#[cfg(test)]
mod session_io_tests {
    use super::*;

    #[test]
    fn open_pages_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "macrodeck-session-test-{}.json",
            std::process::id()
        ));
        let mut pages = HashMap::new();
        pages.insert("profile-1".to_string(), "page-9".to_string());
        write_pages(&path, &pages).expect("write session");
        assert_eq!(load_pages(&path), pages);
        let _ = fs::remove_file(&path);
    }
}
