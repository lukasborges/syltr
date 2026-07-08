//! General settings, persisted as JSON in `<config>/settings.json`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::config_dir;
use crate::spellcheck;

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

fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

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
