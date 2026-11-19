//! StateManager — the single source of truth for the running backend.

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
    config_error: Mutex<Option<String>>,
    started: Instant,
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

        Arc::new(Self {
            app,
            config: RwLock::new(config),
            gaming: AtomicBool::new(gaming),
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
                .map(|p| (p.id.clone(), p.name.clone()))
        };

        let Some((id, name)) = resolved else {
            crate::logging::warn(format!("profile switch target not found: {}", profile));
            return;
        };

        self.set_gaming(name.eq_ignore_ascii_case("Gaming"));

        let mut config = self.config.write().expect("config lock poisoned");
        config.active_profile_id = id;
    }

    pub fn find_button(&self, button_id: &str) -> Option<MacroButton> {
        let config = self.config.read().expect("config lock poisoned");
        config
            .profiles
            .iter()
            .flat_map(|p| p.buttons.iter())
            .find(|b| b.id == button_id)
            .cloned()
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
        Ok(())
    }

    pub fn config_json(&self) -> String {
        self.config
            .read()
            .expect("config lock poisoned")
            .to_json()
            .unwrap_or_else(|_| "{}".to_string())
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
