//! Persistence of the service list and location of the per-service sessions.
//!
//! The list is saved as JSON in `$XDG_CONFIG_HOME/dev.syltr.Syltr/services.json`.
//! Each service has its own session folder in
//! `$XDG_DATA_HOME/dev.syltr.Syltr/sessions/<id>/` (isolated cookies/storage,
//! like the separate "accounts" in Franz).

use gtk::glib;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{catalog, spellcheck};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// Unique id of the instance (based on the service slug, e.g. "whatsapp", "slack-2").
    pub id: String,
    pub name: String,
    pub url: String,
    /// Silences this service's notifications.
    #[serde(default)]
    pub muted: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Languages enabled in the spell checker (empty = disabled).
    #[serde(default)]
    pub spell_languages: Vec<String>,
    /// Media/WebRTC capture (camera, mic, calls). Off by default because it
    /// triggers a PipeWire crash on some systems.
    #[serde(default)]
    pub media_enabled: bool,
}

fn config_dir() -> PathBuf {
    glib::user_config_dir().join(crate::APP_ID)
}

fn services_file() -> PathBuf {
    config_dir().join("services.json")
}

fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

/// Isolated data folder of a service (cookies, localStorage, cache).
pub fn session_dir(id: &str) -> PathBuf {
    glib::user_data_dir()
        .join(crate::APP_ID)
        .join("sessions")
        .join(id)
}

// ---------------------------------------------------------------------------
// Service list
// ---------------------------------------------------------------------------

pub fn load() -> Vec<Service> {
    if let Ok(text) = std::fs::read_to_string(services_file()) {
        if let Ok(list) = serde_json::from_str::<Vec<Service>>(&text) {
            return list;
        }
    }
    // First run: seed with the default services and persist them.
    let defaults = default_services();
    save(&defaults);
    defaults
}

pub fn save(list: &[Service]) {
    let dir = config_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("syltr: could not create {}: {e}", dir.display());
        return;
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => {
            if let Err(e) = std::fs::write(services_file(), json) {
                eprintln!("syltr: failed to save services: {e}");
            }
        }
        Err(e) => eprintln!("syltr: failed to serialize services: {e}"),
    }
}

fn default_services() -> Vec<Service> {
    catalog::DEFAULT_KEYS
        .iter()
        .filter_map(|k| catalog::find(k))
        .map(|e| Service {
            id: e.key.to_string(),
            name: e.name.to_string(),
            url: e.url.to_string(),
            muted: false,
        })
        .collect()
}

/// Generates a unique id from a name/base, avoiding collisions with existing ones.
pub fn make_id(existing: &[Service], base: &str) -> String {
    let slug = slugify(base);
    if !id_taken(existing, &slug) {
        return slug;
    }
    for suffix in 2.. {
        let candidate = format!("{slug}-{suffix}");
        if !id_taken(existing, &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// Lowercases and reduces a string to an ASCII-alphanumeric slug.
fn slugify(base: &str) -> String {
    let slug: String = base
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "service".to_string()
    } else {
        slug.to_string()
    }
}

fn id_taken(existing: &[Service], id: &str) -> bool {
    existing.iter().any(|s| s.id == id)
}

// ---------------------------------------------------------------------------
// General settings
// ---------------------------------------------------------------------------

pub fn load_settings() -> Settings {
    if let Ok(text) = std::fs::read_to_string(settings_file()) {
        if let Ok(settings) = serde_json::from_str::<Settings>(&text) {
            return settings;
        }
    }
    // First run: start with every installed dictionary enabled.
    let settings = Settings {
        spell_languages: spellcheck::default_languages(),
        media_enabled: false,
    };
    save_settings(&settings);
    settings
}

pub fn save_settings(settings: &Settings) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_file(), json);
    }
}

/// Normalizes a URL typed by the user (prepends https:// if the scheme is missing).
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}
