//! User-agent selection.
//!
//! WebKitGTK's default UA advertises WebKit/Safari, which many services that
//! were only tested on Chrome either degrade or block outright. We send a
//! modern Chrome-on-Linux UA globally, and allow per-service overrides
//! (Epiphany-style quirks) for services with a better non-Chrome path — e.g.
//! iCloud, which unlocks its full web app only for Safari.
//!
//! `SYLTR_USER_AGENT`, when set, forces its value for every service.

#[cfg(test)]
mod tests;

const CHROME: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

const SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Safari/605.1.15";

const OVERRIDES: &[(&str, &str)] = &[("icloud.com", SAFARI)];

pub fn for_url(url: &str) -> String {
    if let Ok(forced) = std::env::var("SYLTR_USER_AGENT") {
        return forced;
    }
    OVERRIDES
        .iter()
        .find(|(needle, _)| url.contains(needle))
        .map(|(_, ua)| (*ua).to_string())
        .unwrap_or_else(|| CHROME.to_string())
}
