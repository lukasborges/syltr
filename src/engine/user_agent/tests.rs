use super::{for_url, CHROME, SAFARI};

#[test]
fn defaults_to_chrome() {
    assert_eq!(for_url("https://web.whatsapp.com/"), CHROME);
    assert_eq!(for_url("https://teams.microsoft.com/"), CHROME);
}

#[test]
fn icloud_gets_safari() {
    assert_eq!(for_url("https://www.icloud.com/mail"), SAFARI);
    assert_eq!(for_url("https://icloud.com/"), SAFARI);
}
