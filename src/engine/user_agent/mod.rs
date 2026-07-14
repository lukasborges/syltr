//! User-agent selection.
//!
//! WebKitGTK's default UA advertises WebKit/Safari, which many services that
//! were only tested on Chrome either degrade or block outright. We send a
//! modern Chrome-on-Linux UA globally, allow per-service overrides
//! (Epiphany-style quirks) for services with a better non-Chrome path — e.g.
//! iCloud, which unlocks its full web app only for Safari — and let the user
//! set a custom UA per service in the edit dialog.
//!
//! Precedence: `SYLTR_USER_AGENT` (forces every service) > the service's own
//! custom UA > the built-in quirks table > the Chrome default.

#[cfg(test)]
mod tests;

const CHROME: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

const SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Safari/605.1.15";

const OVERRIDES: &[(&str, &str)] = &[("icloud.com", SAFARI)];

pub fn for_service(url: &str, custom: Option<&str>) -> String {
    if let Ok(forced) = std::env::var("SYLTR_USER_AGENT") {
        return forced;
    }
    match custom.map(str::trim).filter(|s| !s.is_empty()) {
        Some(ua) => ua.to_string(),
        None => from_quirks(url),
    }
}

fn from_quirks(url: &str) -> String {
    OVERRIDES
        .iter()
        .find(|(needle, _)| url.contains(needle))
        .map(|(_, ua)| (*ua).to_string())
        .unwrap_or_else(|| CHROME.to_string())
}
