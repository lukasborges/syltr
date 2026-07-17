//! Isolated per-service network session (cookies, storage, cache) and the
//! download wiring: files go straight to ~/Downloads (or the XDG equivalent)
//! without a dialog, avoiding collisions with existing files.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use gtk::glib;
use gtk::prelude::*;

const DEFAULT_WEB_PROCESS_MEMORY_LIMIT_MB: u32 = 2048;

thread_local! {
    /// All services share one context, matching a normal browser profile while
    /// their `NetworkSession`s keep accounts and storage isolated.
    static WEB_CONTEXT: OnceCell<webkit6::WebContext> = const { OnceCell::new() };
}

/// Shared context with a browser-like memory-pressure policy. WebKit releases
/// caches and other reclaimable data at its conservative/strict thresholds;
/// the kill threshold remains at its safe default of zero, so a background
/// messenger is never discarded merely for crossing the limit.
pub(super) fn context() -> webkit6::WebContext {
    WEB_CONTEXT.with(|cell| {
        cell.get_or_init(|| {
            let mut network_settings = memory_pressure_settings();
            webkit6::NetworkSession::set_memory_pressure_settings(&mut network_settings);

            let web_settings = memory_pressure_settings();
            webkit6::WebContext::builder()
                .memory_pressure_settings(&web_settings)
                .build()
        })
        .clone()
    })
}

fn memory_pressure_settings() -> webkit6::MemoryPressureSettings {
    let mut settings = webkit6::MemoryPressureSettings::new();
    settings.set_memory_limit(configured_memory_limit_mb(
        std::env::var("SYLTR_WEB_PROCESS_MEMORY_MB").ok().as_deref(),
    ));
    // Reclaim caches before the process grows large, but reserve the strict
    // policy for genuinely exceptional usage to avoid repeated full GC cycles.
    settings.set_conservative_threshold(0.40);
    settings.set_strict_threshold(0.70);
    settings
}

fn configured_memory_limit_mb(value: Option<&str>) -> u32 {
    value
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| (1024..=8192).contains(value))
        .unwrap_or(DEFAULT_WEB_PROCESS_MEMORY_LIMIT_MB)
}

/// Builds a persistent, isolated network session under `session_dir` — each
/// service gets its own cookies and storage (like separate accounts).
pub(super) fn build(session_dir: &Path) -> webkit6::NetworkSession {
    let data = session_dir.join("data");
    let cache = session_dir.join("cache");
    let _ = std::fs::create_dir_all(&data);
    let _ = std::fs::create_dir_all(&cache);

    let session = webkit6::NetworkSession::new(data.to_str(), cache.to_str());
    if let Some(cookies) = session.cookie_manager() {
        if let Some(path) = data.join("cookies.sqlite").to_str() {
            cookies.set_persistent_storage(path, webkit6::CookiePersistentStorage::Sqlite);
        }
    }
    // Favicons feed the rail icons.
    if let Some(dm) = session.website_data_manager() {
        dm.set_favicons_enabled(true);
    }
    session
}

/// Routes every download of the session to the downloads directory and posts a
/// desktop notification when it completes.
pub(super) fn wire_downloads(session: &webkit6::NetworkSession) {
    session.connect_download_started(|_, download| {
        download.connect_decide_destination(|download, suggested| {
            let name = if suggested.is_empty() {
                "download"
            } else {
                suggested
            };
            let path = unique_path(&downloads_dir(), name);
            download.set_destination(&path.to_string_lossy());
            true
        });
        download.connect_finished(|download| {
            let Some(path) = download.destination() else {
                return;
            };
            let name = Path::new(path.as_str())
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path.as_str())
                .to_string();
            eprintln!("[syltr] download complete: {path}");
            if let Some(app) = gtk::gio::Application::default() {
                let notif = gtk::gio::Notification::new("Download complete");
                notif.set_body(Some(&name));
                app.send_notification(Some(&format!("syltr-download-{name}")), &notif);
            }
        });
        download.connect_failed(|download, error| {
            eprintln!(
                "[syltr] download failed: {error} — {}",
                download.destination().unwrap_or_default()
            );
        });
    });
}

fn downloads_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Downloads)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads")))
        .unwrap_or_else(std::env::temp_dir)
}

/// A `dir/name` path that does not collide: appends " (1)", " (2)"… if needed.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 1.. {
        let filename = match ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::{configured_memory_limit_mb, DEFAULT_WEB_PROCESS_MEMORY_LIMIT_MB};

    #[test]
    fn memory_limit_uses_balanced_default() {
        assert_eq!(
            configured_memory_limit_mb(None),
            DEFAULT_WEB_PROCESS_MEMORY_LIMIT_MB
        );
        assert_eq!(configured_memory_limit_mb(Some("invalid")), 2048);
        assert_eq!(configured_memory_limit_mb(Some("512")), 2048);
    }

    #[test]
    fn memory_limit_accepts_safe_override() {
        assert_eq!(configured_memory_limit_mb(Some("1024")), 1024);
        assert_eq!(configured_memory_limit_mb(Some("4096")), 4096);
    }
}
