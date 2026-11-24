//! ProfileManager — profile CRUD, assignment and switching without a restart.
//!
//! Every mutation works on a copy of the config and is persisted through the
//! state manager, so a failed validation never leaves memory and disk apart.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::models::{AppConfig, MacroButton, Profile};
use crate::state::AppCore;

fn new_id() -> String {
    format!("{:08x}", rand::thread_rng().gen::<u32>())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn trigger_label(key: i64) -> String {
    match key {
        10 => "/".to_string(),
        11 => "*".to_string(),
        other => format!("Key {}", other),
    }
}

/// A new profile gets the standard 11 numpad slots, unassigned.
fn default_buttons() -> Vec<MacroButton> {
    (1..=11)
        .map(|key| MacroButton {
            id: new_id(),
            key: Some(key),
            name: trigger_label(key),
            enabled: true,
            ..Default::default()
        })
        .collect()
}

fn validate_name(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    if trimmed.chars().count() > 48 {
        return Err("Profile name is too long (48 characters max)".to_string());
    }
    Ok(trimmed)
}

fn name_taken(config: &AppConfig, name: &str, except_id: Option<&str>) -> bool {
    config.profiles.iter().any(|p| {
        p.name.eq_ignore_ascii_case(name) && except_id.map(|id| id != p.id).unwrap_or(true)
    })
}

pub fn create_in(config: &mut AppConfig, name: &str) -> Result<Profile, String> {
    let name = validate_name(name)?;
    if name_taken(config, name, None) {
        return Err(format!("A profile named \"{}\" already exists", name));
    }

    let profile = Profile {
        id: new_id(),
        name: name.to_string(),
        buttons: default_buttons(),
        created_at: now_ms(),
        updated_at: now_ms(),
        ..Default::default()
    };

    if config.active_profile_id.trim().is_empty() {
        config.active_profile_id = profile.id.clone();
    }
    config.profiles.push(profile.clone());
    Ok(profile)
}

pub fn rename_in(config: &mut AppConfig, id: &str, name: &str) -> Result<(), String> {
    let name = validate_name(name)?;
    if name_taken(config, name, Some(id)) {
        return Err(format!("A profile named \"{}\" already exists", name));
    }

    let profile = config
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("Profile {} not found", id))?;

    profile.name = name.to_string();
    profile.updated_at = now_ms();
    Ok(())
}

pub fn duplicate_in(config: &mut AppConfig, id: &str, name: &str) -> Result<Profile, String> {
    let name = validate_name(name)?;
    if name_taken(config, name, None) {
        return Err(format!("A profile named \"{}\" already exists", name));
    }

    let original = config
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("Profile {} not found", id))?;

    let mut copy = original;
    copy.id = new_id();
    copy.name = name.to_string();
    copy.created_at = now_ms();
    copy.updated_at = now_ms();
    for button in &mut copy.buttons {
        button.id = new_id();
    }
    for app in &mut copy.apps {
        app.id = new_id();
    }

    config.profiles.push(copy.clone());
    Ok(copy)
}

pub fn delete_in(config: &mut AppConfig, id: &str) -> Result<(), String> {
    if config.profiles.len() <= 1 {
        return Err("Cannot delete the last profile".to_string());
    }
    if !config.profiles.iter().any(|p| p.id == id) {
        return Err(format!("Profile {} not found", id));
    }

    config.profiles.retain(|p| p.id != id);
    if config.active_profile_id == id {
        config.active_profile_id = config
            .profiles
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
    }
    Ok(())
}

pub fn set_active_in(config: &mut AppConfig, id: &str) -> Result<(), String> {
    if !config.profiles.iter().any(|p| p.id == id) {
        return Err(format!("Profile {} not found", id));
    }
    config.active_profile_id = id.to_string();
    Ok(())
}

// ---- state-aware wrappers -------------------------------------------------

pub fn create(core: &Arc<AppCore>, name: &str) -> Result<Profile, String> {
    let mut config = core.config();
    let profile = create_in(&mut config, name)?;
    core.set_config(config)?;
    Ok(profile)
}

pub fn rename(core: &Arc<AppCore>, id: &str, name: &str) -> Result<(), String> {
    let mut config = core.config();
    rename_in(&mut config, id, name)?;
    core.set_config(config)
}

pub fn duplicate(core: &Arc<AppCore>, id: &str, name: &str) -> Result<Profile, String> {
    let mut config = core.config();
    let profile = duplicate_in(&mut config, id, name)?;
    core.set_config(config)?;
    Ok(profile)
}

pub fn delete(core: &Arc<AppCore>, id: &str) -> Result<(), String> {
    let mut config = core.config();
    delete_in(&mut config, id)?;
    core.set_config(config)
}

pub fn set_active(core: &Arc<AppCore>, id: &str) -> Result<(), String> {
    let mut config = core.config();
    set_active_in(&mut config, id)?;
    let apps = config.profile(id).map(|p| p.apps.clone()).unwrap_or_default();
    core.set_config(config)?;
    crate::apps::apply(&apps);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_default() -> AppConfig {
        let mut config = AppConfig::default();
        create_in(&mut config, "Normal").unwrap();
        config
    }

    #[test]
    fn creates_a_profile_with_eleven_slots() {
        let mut config = AppConfig::default();
        let profile = create_in(&mut config, "Gaming").unwrap();
        assert_eq!(profile.buttons.len(), 11);
        assert_eq!(profile.buttons[9].name, "/");
        assert_eq!(profile.buttons[10].name, "*");
        assert_eq!(config.active_profile_id, profile.id, "first profile becomes active");
    }

    #[test]
    fn rejects_empty_and_duplicate_names() {
        let mut config = config_with_default();
        assert!(create_in(&mut config, "   ").is_err());
        assert!(create_in(&mut config, "normal").is_err());
        assert!(create_in(&mut config, "Gaming").is_ok());
    }

    #[test]
    fn renames_and_keeps_names_unique() {
        let mut config = config_with_default();
        let gaming = create_in(&mut config, "Gaming").unwrap();
        assert!(rename_in(&mut config, &gaming.id, "normal").is_err());
        rename_in(&mut config, &gaming.id, "Games").unwrap();
        assert_eq!(config.profiles[1].name, "Games");
        assert!(rename_in(&mut config, "missing", "X").is_err());
    }

    #[test]
    fn duplicate_gets_fresh_ids() {
        let mut config = config_with_default();
        let original = config.profiles[0].clone();
        let copy = duplicate_in(&mut config, &original.id, "Normal Copy").unwrap();

        assert_ne!(copy.id, original.id);
        assert_eq!(copy.buttons.len(), original.buttons.len());
        for (a, b) in copy.buttons.iter().zip(original.buttons.iter()) {
            assert_ne!(a.id, b.id);
        }
        copy.validate().expect("duplicate is valid");
    }

    #[test]
    fn delete_reassigns_active_and_keeps_last_profile() {
        let mut config = config_with_default();
        let gaming = create_in(&mut config, "Gaming").unwrap();
        set_active_in(&mut config, &gaming.id).unwrap();

        delete_in(&mut config, &gaming.id).unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.active_profile_id, config.profiles[0].id);

        let last_id = config.profiles[0].id.clone();
        assert!(delete_in(&mut config, &last_id).is_err());
    }

    #[test]
    fn switching_profiles_does_not_touch_buttons() {
        let mut config = config_with_default();
        let gaming = create_in(&mut config, "Gaming").unwrap();
        let before = config.profiles[0].buttons.clone();
        set_active_in(&mut config, &gaming.id).unwrap();
        assert_eq!(config.profiles[0].buttons.len(), before.len());
        assert_eq!(config.profiles[0].buttons[0].id, before[0].id);
    }
}
