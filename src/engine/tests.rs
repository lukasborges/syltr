use super::media_capture_enabled;

#[test]
fn teams_calls_are_opt_in() {
    assert!(!media_capture_enabled(
        "https://teams.microsoft.com/",
        false
    ));
    assert!(media_capture_enabled("https://teams.microsoft.com/", true));
}

#[test]
fn other_services_keep_media_capture() {
    assert!(media_capture_enabled("https://web.whatsapp.com/", false));
    assert!(media_capture_enabled("https://chat.google.com/", false));
}
