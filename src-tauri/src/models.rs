//! Typed configuration model.
//!
//! The on-disk `config.json` is still read by the React editor, so every struct
//! keeps unknown fields (`#[serde(flatten)] extra`) and keeps `version: 1`. This
//! module is the single typed view the native backend works with.

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value};

pub type JsonObject = JsonMap<String, Value>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub active_profile_id: String,
    pub profiles: Vec<Profile>,
    pub settings: Settings,
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            active_profile_id: String::new(),
            profiles: Vec::new(),
            settings: Settings::default(),
            extra: JsonObject::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// Additive visual layout: `numpad`, `numpadNav` or `full`.
    /// Missing/invalid values are migrated to `numpad`; never removed.
    pub layout: Option<String>,
    pub buttons: Vec<MacroButton>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            layout: None,
            buttons: Vec::new(),
            created_at: 0,
            updated_at: 0,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MacroButton {
    pub id: String,
    /// Legacy numpad slot (1-9, 10 = `/`, 11 = `*`). Kept verbatim so old
    /// assignments continue to work after layout migration.
    pub key: Option<i64>,
    pub key_name: Option<String>,
    /// Stable physical identity from Raw Input scan code + extended flag.
    pub physical_key_id: Option<String>,
    pub name: String,
    pub icon: String,
    pub image_path: Option<String>,
    pub description: String,
    pub enabled: bool,
    pub background_color: Option<String>,
    pub icon_color: Option<String>,
    pub actions: Vec<ButtonAction>,
    /// Native trigger, e.g. `"Numpad1"` or `"Ctrl+Shift+Numpad4"`.
    /// Falls back to `key` when missing.
    pub trigger: Option<String>,
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for MacroButton {
    fn default() -> Self {
        Self {
            id: String::new(),
            key: None,
            key_name: None,
            physical_key_id: None,
            name: String::new(),
            icon: String::new(),
            image_path: None,
            description: String::new(),
            enabled: true,
            background_color: None,
            icon_color: None,
            actions: Vec::new(),
            trigger: None,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ButtonAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub name: String,
    pub params: JsonObject,
    pub delay_after: Option<u64>,
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for ButtonAction {
    fn default() -> Self {
        Self {
            id: String::new(),
            action_type: String::new(),
            name: String::new(),
            params: JsonObject::new(),
            delay_after: None,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub keyboard: KeyboardSettings,
    #[serde(flatten)]
    pub extra: JsonObject,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyboardSettings {
    pub ahk_executable_path: String,
    pub ahk_script_path: String,
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            ahk_executable_path: String::new(),
            ahk_script_path: String::new(),
            extra: JsonObject::new(),
        }
    }
}

impl AppConfig {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let trimmed = json.trim();
        if trimmed.is_empty() || trimmed == "{}" {
            return Ok(Self::default());
        }
        serde_json::from_str(json).map_err(|e| format!("Invalid config JSON: {}", e))
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Cannot serialize config: {}", e))
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile_by_name(&self, name: &str) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Profile the keyboard engine acts on. `Gaming`/`Normal` follow the mode
    /// flag, but an explicitly selected custom profile (e.g. "Work") wins so a
    /// button can jump to any profile, not just the mode pair.
    pub fn active_profile(&self, gaming: bool) -> Option<&Profile> {
        if let Some(active) = self.profile(&self.active_profile_id) {
            let is_mode_profile = active.name.eq_ignore_ascii_case("Gaming")
                || active.name.eq_ignore_ascii_case("Normal");
            if !is_mode_profile {
                return Some(active);
            }
        }

        self.profile_by_name(if gaming { "Gaming" } else { "Normal" })
            .or_else(|| self.profile(&self.active_profile_id))
            .or_else(|| self.profiles.first())
    }

    /// Adds layout and physical-key fields without altering existing actions,
    /// legacy slots or unknown fields. Returns true when anything changed.
    pub fn migrate_layouts(&mut self) -> bool {
        let mut changed = false;
        for profile in &mut self.profiles {
            let valid_layout = matches!(
                profile.layout.as_deref(),
                Some("full") | Some("numpad") | Some("numpadNav")
            );
            if !valid_layout {
                profile.layout = Some("numpad".to_string());
                changed = true;
            }

            for button in &mut profile.buttons {
                let has_id = button
                    .physical_key_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .is_some();
                if has_id {
                    continue;
                }

                let from_trigger = button
                    .trigger
                    .as_deref()
                    .and_then(|trigger| crate::keyid::KeyId::parse(trigger.trim()));
                let physical = from_trigger.or_else(|| {
                    button.key.and_then(crate::keyid::KeyId::from_legacy_key)
                });
                if let Some(physical) = physical {
                    button.physical_key_id = Some(physical.id());
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.profiles.is_empty() {
            return Ok(());
        }

        let mut names = std::collections::HashSet::new();
        for profile in &self.profiles {
            if profile.id.trim().is_empty() {
                return Err(format!("Profile \"{}\" has an empty id", profile.name));
            }
            if !names.insert(profile.id.as_str()) {
                return Err(format!("Duplicate profile id: {}", profile.id));
            }
            if profile.name.trim().is_empty() {
                return Err(format!("Profile {} has an empty name", profile.id));
            }
            profile.validate()?;
        }

        if !self.active_profile_id.trim().is_empty()
            && self.profile(&self.active_profile_id).is_none()
        {
            return Err(format!(
                "activeProfileId {} does not match any profile",
                self.active_profile_id
            ));
        }

        Ok(())
    }
}

impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        let mut button_ids = std::collections::HashSet::new();
        let mut physical_ids = std::collections::HashSet::new();
        let mut triggers: Vec<String> = Vec::new();

        for button in &self.buttons {
            if button.id.trim().is_empty() {
                return Err(format!(
                    "Profile \"{}\" has a button with an empty id",
                    self.name
                ));
            }
            if !button_ids.insert(button.id.as_str()) {
                return Err(format!(
                    "Profile \"{}\" has duplicate button id {}",
                    self.name, button.id
                ));
            }

            if let Some(id) = button.physical_key_id.as_deref().map(str::trim).filter(|id| !id.is_empty()) {
                if !physical_ids.insert(id) {
                    return Err(format!(
                        "Profile \"{}\" binds physical key {} twice",
                        self.name, id
                    ));
                }
            }

            if let Some(trigger) = button.effective_trigger()? {
                let label = trigger.to_string();
                if triggers.iter().any(|t| t.eq_ignore_ascii_case(&label)) {
                    return Err(format!(
                        "Profile \"{}\" binds {} twice",
                        self.name, label
                    ));
                }
                triggers.push(label);
            }

            for action in &button.actions {
                crate::actions::validate_type(&action.action_type).map_err(|e| {
                        format!(
                            "Profile \"{}\" button \"{}\": {}",
                            self.name, button.name, e
                        )
                    })?;
            }
        }

        Ok(())
    }
}

impl MacroButton {
    /// Trigger explicitly set on the button, or derived from the legacy slot.
    pub fn effective_trigger(&self) -> Result<Option<crate::shortcut::Shortcut>, String> {
        if let Some(trigger) = self.trigger.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            return crate::shortcut::Shortcut::parse(trigger).map(Some);
        }
        Ok(self.key.and_then(crate::shortcut::Shortcut::from_legacy_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(id: &str, key: i64) -> MacroButton {
        MacroButton {
            id: id.to_string(),
            key: Some(key),
            name: format!("Button {}", id),
            ..Default::default()
        }
    }

    fn profile(id: &str, name: &str, keys: Vec<i64>) -> Profile {
        Profile {
            id: id.to_string(),
            name: name.to_string(),
            buttons: keys
                .iter()
                .enumerate()
                .map(|(i, k)| button(&format!("{}-{}", id, i), *k))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn parses_legacy_and_keeps_unknown_fields() {
        let json = r#"{
            "version": 1,
            "activeProfileId": "a",
            "profiles": [{"id":"a","name":"Normal","buttons":[],"createdAt":1,"updatedAt":2,"mystery":true}],
            "settings": {"keyboard": {"ahkExecutablePath":"x"}, "audio": {"soundVolumeViewPath":"y"}}
        }"#;

        let config = AppConfig::from_json(json).expect("config parses");
        assert_eq!(config.version, 1);
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(
            config.profiles[0].extra.get("mystery"),
            Some(&Value::Bool(true))
        );

        let round_trip: Value =
            serde_json::from_str(&config.to_json().unwrap()).expect("valid json");
        assert_eq!(
            round_trip["settings"]["audio"]["soundVolumeViewPath"], "y",
            "unknown settings must survive a round trip"
        );
    }

    #[test]
    fn layout_migration_is_additive_and_preserves_old_fields() {
        let mut config = AppConfig::from_json(
            r#"{
                "version": 1,
                "activeProfileId": "a",
                "profiles": [{
                    "id":"a","name":"Normal","legacyMarker":7,
                    "buttons":[
                        {"id":"b1","key":1,"name":"One","mystery":"keep"},
                        {"id":"b2","key":10,"name":"Slash","trigger":"NumpadDivide"},
                        {"id":"b3","name":"Unsupported","actions":[]}
                    ]
                }],
                "settings": {}
            }"#,
        )
        .unwrap();

        assert!(config.migrate_layouts());
        assert!(!config.migrate_layouts(), "migration must be idempotent");

        let profile = &config.profiles[0];
        assert_eq!(profile.layout.as_deref(), Some("numpad"));
        assert_eq!(profile.extra.get("legacyMarker"), Some(&Value::from(7)));
        assert_eq!(
            profile.buttons[0].physical_key_id.as_deref(),
            Some("Numpad1")
        );
        assert_eq!(
            profile.buttons[1].physical_key_id.as_deref(),
            Some("NumpadDivide")
        );
        assert_eq!(profile.buttons[2].physical_key_id, None);
        assert_eq!(
            profile.buttons[0].extra.get("mystery"),
            Some(&Value::String("keep".into()))
        );
        assert_eq!(profile.buttons[0].key, Some(1), "legacy slot preserved");
    }

    #[test]
    fn empty_config_is_valid() {
        let config = AppConfig::from_json("{}").unwrap();
        assert!(config.profiles.is_empty());
        config.validate().expect("empty config is valid");
    }

    #[test]
    fn active_profile_follows_gaming_mode() {
        let mut config = AppConfig {
            active_profile_id: "n".into(),
            profiles: vec![profile("g", "Gaming", vec![1]), profile("n", "Normal", vec![1])],
            ..Default::default()
        };

        assert_eq!(config.active_profile(true).unwrap().name, "Gaming");
        assert_eq!(config.active_profile(false).unwrap().name, "Normal");

        config.profiles[0].name = "Games".into();
        assert_eq!(
            config.active_profile(true).unwrap().id,
            "n",
            "falls back to activeProfileId when no Gaming profile exists"
        );
    }

    #[test]
    fn active_profile_honors_selected_custom_profile() {
        let mut config = AppConfig {
            active_profile_id: "work".into(),
            profiles: vec![
                profile("g", "Gaming", vec![1]),
                profile("n", "Normal", vec![1]),
                profile("work", "Work", vec![1]),
            ],
            ..Default::default()
        };

        assert_eq!(config.active_profile(true).unwrap().name, "Work");
        assert_eq!(config.active_profile(false).unwrap().name, "Work");

        config.active_profile_id = "n".into();
        assert_eq!(
            config.active_profile(true).unwrap().name,
            "Gaming",
            "mode selection resumes once a mode profile is active"
        );
        assert_eq!(config.active_profile(false).unwrap().name, "Normal");
    }

    #[test]
    fn duplicate_triggers_are_rejected() {
        let config = AppConfig {
            profiles: vec![profile("p", "Normal", vec![1, 1])],
            ..Default::default()
        };
        let error = config.validate().unwrap_err();
        assert!(error.contains("binds"), "unexpected error: {}", error);
    }

    #[test]
    fn unknown_action_type_is_rejected() {
        let mut p = profile("p", "Normal", vec![1]);
        p.buttons[0].actions.push(ButtonAction {
            action_type: "not_a_real_action".into(),
            ..Default::default()
        });
        let error = p.validate().unwrap_err();
        assert!(error.contains("Unknown action"), "unexpected: {}", error);
    }
}
