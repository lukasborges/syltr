//! The service list, persisted as JSON in `<config>/services.json`.

use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::PathBuf;

use super::config_dir;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// Unique instance id (based on the service slug, e.g. "whatsapp", "slack-2").
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub muted: bool,
    /// A disabled service stays in the list but loads no web view (no resources,
    /// no notifications) until re-enabled from its context menu.
    #[serde(default)]
    pub disabled: bool,
    /// Custom user-agent for this service; falls back to the built-in resolver
    /// (see `engine::user_agent`) when `None` or empty.
    #[serde(default)]
    pub user_agent: Option<String>,
}

pub struct LoadedServices {
    pub services: Vec<Service>,
    pub first_run: bool,
}

fn services_file() -> PathBuf {
    config_dir().join("services.json")
}

pub fn load() -> LoadedServices {
    match std::fs::read_to_string(services_file()) {
        Ok(text) => match serde_json::from_str::<Vec<Service>>(&text) {
            Ok(services) => LoadedServices {
                services,
                first_run: false,
            },
            Err(e) => {
                eprintln!("syltr: failed to parse services, starting empty: {e}");
                LoadedServices {
                    services: Vec::new(),
                    first_run: false,
                }
            }
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let services = Vec::new();
            save(&services);
            LoadedServices {
                services,
                first_run: true,
            }
        }
        Err(e) => {
            eprintln!("syltr: failed to read services, starting empty: {e}");
            LoadedServices {
                services: Vec::new(),
                first_run: false,
            }
        }
    }
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
