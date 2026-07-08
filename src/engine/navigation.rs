//! Policy for external links: a link the user clicks to an unrelated site opens
//! in the system browser, while same-site navigation and auth (SSO) flows —
//! which legitimately cross domains and redirect back — stay in-app.

use gtk::gio;

/// Registrable domains of identity providers that SSO flows reach across
/// domains. Same-site navigation is already kept in-app, so the services' own
/// domains (google/microsoft) need no entry; `facebook.com` is here for
/// Messenger's login (messenger.com).
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

/// Path fragments typical of SSO endpoints (Keycloak, ADFS, OIDC, SAML), so a
/// corporate identity provider on an arbitrary domain stays in-app.
const AUTH_PATH_FRAGMENTS: &[&str] = &[
    "/auth/realms/",             // Keycloak
    "/protocol/saml",            // Keycloak SAML
    "/protocol/openid-connect",  // Keycloak OIDC
    "/oauth2/",
    "/oauth/authorize",
    "/openid-connect/",
    "/saml2/",
    "/simplesaml/",              // SimpleSAMLphp
    "/adfs/",
    "/sso/",
];

/// Whether `target` should open in the system browser rather than in the app.
/// `home` is the service's URL and `current` is the page currently loaded in the
/// frame (both keep their site in-app, so multi-step auth flows on a corporate
/// domain don't pop out). Non-web schemes (mailto:, tel:…) always go out.
pub(crate) fn should_open_externally(target: &str, home: &str, current: Option<&str>) -> bool {
    if !is_web_url(target) {
        return true;
    }
    let Some(target_host) = host_of(target) else {
        return false;
    };
    if same_site(&target_host, home) {
        return false;
    }
    if current.is_some_and(|c| same_site(&target_host, c)) {
        return false;
    }
    if is_auth_host(&target_host) || is_auth_path(target) {
        return false;
    }
    true
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

/// Whether `target_host` shares a registrable domain with the host in `url`.
fn same_site(target_host: &str, url: &str) -> bool {
    host_of(url).is_some_and(|h| registrable_domain(&h) == registrable_domain(target_host))
}

fn is_auth_host(host: &str) -> bool {
    AUTH_DOMAINS.contains(&registrable_domain(host))
}

fn is_auth_path(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    AUTH_PATH_FRAGMENTS.iter().any(|f| path.contains(f))
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
    use super::should_open_externally;

    const WHATSAPP: &str = "https://web.whatsapp.com/";

    fn external(target: &str) -> bool {
        should_open_externally(target, WHATSAPP, Some(WHATSAPP))
    }

    #[test]
    fn same_site_and_subdomains_stay_in_app() {
        assert!(!external("https://web.whatsapp.com/send"));
        assert!(!external("https://whatsapp.com/faq"));
    }

    #[test]
    fn unrelated_sites_are_external() {
        assert!(external("https://example.com/x"));
        assert!(external("https://youtube.com/watch"));
    }

    #[test]
    fn known_identity_providers_stay_in_app() {
        // Same registrable domain (google.com).
        assert!(!should_open_externally(
            "https://accounts.google.com/o/oauth2/auth",
            "https://chat.google.com/",
            Some("https://chat.google.com/"),
        ));
        // Cross-domain provider on the allowlist.
        assert!(!should_open_externally(
            "https://login.microsoftonline.com/common/oauth2",
            "https://teams.microsoft.com/",
            Some("https://teams.microsoft.com/"),
        ));
    }

    #[test]
    fn corporate_sso_on_arbitrary_domain_stays_in_app() {
        // Keycloak SAML endpoint reached from a Google service.
        assert!(!should_open_externally(
            "https://sso.mycompany.com/auth/realms/acme/protocol/saml",
            "https://chat.google.com/",
            Some("https://accounts.google.com/signin"),
        ));
        // A further click while on the corporate domain stays in-app.
        assert!(!should_open_externally(
            "https://sso.mycompany.com/account/password",
            "https://chat.google.com/",
            Some("https://sso.mycompany.com/auth/realms/acme"),
        ));
    }

    #[test]
    fn non_web_schemes_are_external() {
        assert!(external("mailto:someone@example.com"));
        assert!(external("tel:+15551234"));
    }
}
