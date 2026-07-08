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

/// The URL to open in the system browser for a navigation to `target`, or
/// `None` to keep it in-app. Unwraps link redirectors (e.g. Google's
/// `google.com/url?q=…`) so the real destination is judged and opened.
pub(crate) fn external_target(target: &str, home: &str, current: Option<&str>) -> Option<String> {
    let dest = unwrap_redirect(target).unwrap_or_else(|| target.to_string());
    should_open_externally(&dest, home, current).then_some(dest)
}

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

/// Unwraps a link redirector to its real destination. Google wraps external
/// links clicked in Chat/Gmail/Docs as `google.com/url?q=<target>`, which is
/// same-site and then 302s away — so we resolve the target up front.
fn unwrap_redirect(url: &str) -> Option<String> {
    let host = host_of(url)?;
    if registrable_domain(&host) != "google.com" || !url.contains("/url?") {
        return None;
    }
    let target = query_param(url, "q").or_else(|| query_param(url, "url"))?;
    is_web_url(&target).then_some(target)
}

/// Value of query parameter `key`, percent-decoded.
fn query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
