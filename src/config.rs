//! Persistência da lista de serviços e localização das sessões por serviço.
//!
//! A lista é salva como JSON em `$XDG_CONFIG_HOME/dev.syltr.Syltr/services.json`.
//! Cada serviço tem sua própria pasta de sessão em
//! `$XDG_DATA_HOME/dev.syltr.Syltr/sessions/<id>/` (cookies/armazenamento
//! isolados, como as "contas" separadas do Franz).

use gtk::glib;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::catalog;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// id único da instância (base do slug do serviço, ex. "whatsapp", "slack-2")
    pub id: String,
    pub name: String,
    pub url: String,
    /// silencia as notificações desse serviço
    #[serde(default)]
    pub muted: bool,
}

fn config_dir() -> PathBuf {
    glib::user_config_dir().join(crate::APP_ID)
}

fn services_file() -> PathBuf {
    config_dir().join("services.json")
}

/// Pasta de dados isolada de um serviço (cookies, localStorage, cache).
pub fn session_dir(id: &str) -> PathBuf {
    glib::user_data_dir()
        .join(crate::APP_ID)
        .join("sessions")
        .join(id)
}

pub fn load() -> Vec<Service> {
    if let Ok(text) = std::fs::read_to_string(services_file()) {
        if let Ok(list) = serde_json::from_str::<Vec<Service>>(&text) {
            return list;
        }
    }
    // Primeira execução: semeia com os serviços padrão e grava.
    let defaults = default_services();
    save(&defaults);
    defaults
}

pub fn save(list: &[Service]) {
    let dir = config_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("syltr: não consegui criar {}: {e}", dir.display());
        return;
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => {
            if let Err(e) = std::fs::write(services_file(), json) {
                eprintln!("syltr: falha ao salvar serviços: {e}");
            }
        }
        Err(e) => eprintln!("syltr: falha ao serializar serviços: {e}"),
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

/// Gera um id único a partir de um nome/base, evitando colisão com os existentes.
pub fn make_id(existing: &[Service], base: &str) -> String {
    let slug: String = base
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    let slug = if slug.is_empty() { "servico".to_string() } else { slug };

    if !existing.iter().any(|s| s.id == slug) {
        return slug;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{slug}-{n}");
        if !existing.iter().any(|s| s.id == candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Normaliza uma URL digitada pelo usuário (adiciona https:// se faltar esquema).
pub fn normalize_url(input: &str) -> String {
    let t = input.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("https://{t}")
    }
}
