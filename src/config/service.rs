//! The service list, persisted as JSON in `<config>/services.json`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::config_dir;
use crate::catalog;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// Unique instance id (based on the service slug, e.g. "whatsapp", "slack-2").
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub muted: bool,
}

fn services_file() -> PathBuf {
    config_dir().join("services.json")
}

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
