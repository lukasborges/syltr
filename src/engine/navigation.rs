//! Policy for external links: a link the user clicks to an unrelated site opens
//! in the system browser, while same-site navigation and auth (SSO) flows —
//! which legitimately cross domains and redirect back — stay in-app.

use gtk::gio;

/// Registrable domains of identity providers that SSO flows reach across
/// domains. Listed only for providers whose domain differs from the services'
/// own (same-site navigation is already kept in-app, so google/microsoft need
/// no entry). `facebook.com` is here for Messenger's login (messenger.com).
const AUTH_DOMAINS: &[&str] = &[
    "microsoftonline.com",
    "live.com",
    "apple.com",
    "okta.com",
    "auth0.com",
    "onelogin.com",
    "duosecurity.com",
    "pingidentity.com",
    "facebook.com",
];

/// Whether `target` should open in the system browser rather than in `home`'s
/// view. Non-web schemes (mailto:, tel:…) always go out; web URLs go out only
/// when they leave the service's site and are not an auth provider.
pub(crate) fn is_external(target: &str, home: &str) -> bool {
    if !is_web_url(target) {
        return true;
    }
    let (Some(target_host), Some(home_host)) = (host_of(target), host_of(home)) else {
        return false;
    };
    registrable_domain(&target_host) != registrable_domain(&home_host)
        && !is_auth_host(&target_host)
}

/// Opens `url` in the user's default application (browser, mail client…).
pub(crate) fn open_external(url: &str) {
    if let Err(e) = gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>) {
        eprintln!("[syltr] could not open {url} externally: {e}");
    }
}

fn is_web_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn is_auth_host(host: &str) -> bool {
    let domain = registrable_domain(host);
    AUTH_DOMAINS.contains(&domain)
}

/// Extracts the lowercase host from a URL (no scheme, userinfo or port).
fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let authority = after_scheme.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The last two labels of a host (e.g. "web.whatsapp.com" -> "whatsapp.com").
/// An approximation of the registrable domain — good enough for the target
/// services, without pulling in a public-suffix list.
fn registrable_domain(host: &str) -> &str {
    match host.rmatch_indices('.').nth(1) {
        Some((i, _)) => &host[i + 1..],
        None => host,
    }
}

#[cfg(test)]
mod tests {
    use super::is_external;

    const WHATSAPP: &str = "https://web.whatsapp.com/";

    #[test]
    fn same_site_and_subdomains_stay_in_app() {
        assert!(!is_external("https://web.whatsapp.com/send", WHATSAPP));
        assert!(!is_external("https://whatsapp.com/faq", WHATSAPP));
    }

    #[test]
    fn unrelated_sites_are_external() {
        assert!(is_external("https://example.com/x", WHATSAPP));
        assert!(is_external("https://youtube.com/watch", WHATSAPP));
    }

    #[test]
    fn sso_redirect_targets_stay_in_app() {
        // Same registrable domain (google.com).
        assert!(!is_external(
            "https://accounts.google.com/o/oauth2/auth",
            "https://chat.google.com/",
        ));
        // Cross-domain identity provider on the allowlist.
        assert!(!is_external(
            "https://login.microsoftonline.com/common/oauth2",
            "https://teams.microsoft.com/",
        ));
    }

    #[test]
    fn non_web_schemes_are_external() {
        assert!(is_external("mailto:someone@example.com", WHATSAPP));
        assert!(is_external("tel:+15551234", WHATSAPP));
    }
}
