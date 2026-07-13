//! Unread count extracted from the page title.

#[cfg(test)]
mod tests;

/// Extracts the unread count from a title (e.g. "(5) WhatsApp", "Inbox (12) …",
/// "5 messages"). Returns 0 when none is found.
pub(super) fn from_title(title: &str) -> u32 {
    let bytes = title.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if matches!(c, b'(' | b'[' | b'{') {
            if let Some(n) = leading_number(&bytes[i + 1..]) {
                return n;
            }
        }
    }
    leading_number(bytes).unwrap_or(0)
}

/// Parses the run of ASCII digits at the start of `bytes`, if any.
fn leading_number(bytes: &[u8]) -> Option<u32> {
    let mut n = 0u32;
    let mut found = false;
    for &d in bytes {
        if !d.is_ascii_digit() {
            break;
        }
        n = n.saturating_mul(10).saturating_add((d - b'0') as u32);
        found = true;
    }
    found.then_some(n)
}
