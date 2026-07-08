//! Configuration: the service list, general settings, and where they live on
//! disk. Persistence lives in the `service` and `settings` submodules; this file
//! holds the shared paths and URL handling.
//!
//! Each service has its own session folder under
//! `$XDG_DATA_HOME/dev.syltr.Syltr/sessions/<id>/` (isolated cookies/storage,
//! like the separate "accounts" in Franz).

mod service;
mod settings;

pub use service::{load, make_id, save, Service};
pub use settings::{load_settings, save_settings, Settings};

use gtk::glib;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    glib::user_config_dir().join(crate::APP_ID)
}

/// Isolated data folder of a service (cookies, localStorage, cache).
pub fn session_dir(id: &str) -> PathBuf {
    glib::user_data_dir()
        .join(crate::APP_ID)
        .join("sessions")
        .join(id)
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
