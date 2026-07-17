use super::{for_service, CHROME, SAFARI};

#[test]
fn defaults_to_chrome() {
    assert_eq!(for_service("https://web.whatsapp.com/", None), CHROME);
    assert_eq!(for_service("https://teams.microsoft.com/", None), CHROME);
}

#[test]
fn icloud_gets_safari() {
    assert_eq!(for_service("https://www.icloud.com/mail", None), SAFARI);
    assert_eq!(for_service("https://icloud.com/", None), SAFARI);
}

#[test]
fn google_calendar_gets_safari() {
    assert_eq!(for_service("https://calendar.google.com/", None), SAFARI);
}

#[test]
fn custom_overrides_quirks_and_default() {
    assert_eq!(
        for_service("https://web.whatsapp.com/", Some("MyUA")),
        "MyUA"
    );
    assert_eq!(
        for_service("https://www.icloud.com/mail", Some("MyUA")),
        "MyUA"
    );
}

#[test]
fn blank_custom_falls_back() {
    assert_eq!(
        for_service("https://web.whatsapp.com/", Some("   ")),
        CHROME
    );
    assert_eq!(for_service("https://web.whatsapp.com/", Some("")), CHROME);
}
