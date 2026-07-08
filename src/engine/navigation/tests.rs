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
