use super::from_title;

#[test]
fn parenthesized_count() {
    assert_eq!(from_title("(5) WhatsApp"), 5);
    assert_eq!(from_title("Inbox (12) - Gmail"), 12);
    assert_eq!(from_title("[3] Slack"), 3);
    assert_eq!(from_title("{7} Chat"), 7);
}

#[test]
fn leading_count() {
    assert_eq!(from_title("5 messages"), 5);
}

#[test]
fn no_count() {
    assert_eq!(from_title("WhatsApp"), 0);
    assert_eq!(from_title(""), 0);
    assert_eq!(from_title("Chat (beta)"), 0);
}

#[test]
fn plus_suffix_and_large_counts() {
    assert_eq!(from_title("(99+) Teams"), 99);
    assert_eq!(from_title("(4294967296) overflow"), u32::MAX);
}
