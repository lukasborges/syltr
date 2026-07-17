//! Catalog of known services ("recipes").
//!
//! It is just a static table used by the "Add service" dialog. Any custom URL
//! is accepted too — the catalog only lists popular services with a stable web
//! URL as shortcuts (self-hosted/custom instances go in via a custom URL).

pub struct Entry {
    /// Stable key used as the base of the instance id.
    pub key: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    /// Group shown in the "Add service" dialog.
    pub category: &'static str,
    /// Whether new instances should keep running when they are not selected.
    pub background_by_default: bool,
}

pub const CATALOG: &[Entry] = &[
    background_entry(
        "whatsapp",
        "WhatsApp Web",
        "https://web.whatsapp.com/",
        "Messaging",
    ),
    background_entry(
        "telegram",
        "Telegram Web",
        "https://web.telegram.org/",
        "Messaging",
    ),
    background_entry(
        "messenger",
        "Messenger",
        "https://www.messenger.com/",
        "Messaging",
    ),
    entry(
        "slack",
        "Slack",
        "https://app.slack.com/client",
        "Messaging",
    ),
    entry("discord", "Discord", "https://discord.com/app", "Messaging"),
    background_entry(
        "element",
        "Element (Matrix)",
        "https://app.element.io/",
        "Messaging",
    ),
    entry("skype", "Skype", "https://web.skype.com/", "Messaging"),
    background_entry(
        "gmessages",
        "Google Messages",
        "https://messages.google.com/web/",
        "Messaging",
    ),
    background_entry(
        "threema",
        "Threema Web",
        "https://web.threema.ch/",
        "Messaging",
    ),
    background_entry(
        "groupme",
        "GroupMe",
        "https://web.groupme.com/",
        "Messaging",
    ),
    entry(
        "instagram",
        "Instagram",
        "https://www.instagram.com/direct/inbox/",
        "Messaging",
    ),
    entry(
        "linkedin",
        "LinkedIn",
        "https://www.linkedin.com/messaging/",
        "Messaging",
    ),
    entry("x", "X", "https://x.com/messages", "Messaging"),
    entry(
        "gchat",
        "Google Chat",
        "https://chat.google.com/",
        "Messaging",
    ),
    entry(
        "teams",
        "Microsoft Teams",
        "https://teams.microsoft.com/",
        "Messaging",
    ),
    entry("gmail", "Gmail", "https://mail.google.com/", "Email"),
    entry(
        "outlook",
        "Outlook",
        "https://outlook.live.com/mail/",
        "Email",
    ),
    entry("proton", "Proton Mail", "https://mail.proton.me/", "Email"),
    entry("tuta", "Tuta (Tutanota)", "https://app.tuta.com/", "Email"),
    entry("fastmail", "Fastmail", "https://app.fastmail.com/", "Email"),
    entry("zohomail", "Zoho Mail", "https://mail.zoho.com/", "Email"),
    entry(
        "yahoomail",
        "Yahoo Mail",
        "https://mail.yahoo.com/",
        "Email",
    ),
    entry(
        "icloud",
        "iCloud Mail",
        "https://www.icloud.com/mail",
        "Email",
    ),
    entry(
        "gcalendar",
        "Google Calendar",
        "https://calendar.google.com/",
        "Calendar",
    ),
    entry(
        "mstodo",
        "Microsoft To Do",
        "https://to-do.office.com/",
        "Tasks",
    ),
    entry("todoist", "Todoist", "https://app.todoist.com/", "Tasks"),
    entry("trello", "Trello", "https://trello.com/", "Tasks"),
    entry("asana", "Asana", "https://app.asana.com/", "Tasks"),
    entry("clickup", "ClickUp", "https://app.clickup.com/", "Tasks"),
    entry("chatgpt", "ChatGPT", "https://chatgpt.com/", "AI"),
    entry("claude", "Claude", "https://claude.ai/", "AI"),
    entry("gemini", "Gemini", "https://gemini.google.com/", "AI"),
    entry(
        "copilot",
        "Microsoft Copilot",
        "https://copilot.microsoft.com/",
        "AI",
    ),
    entry("deepseek", "DeepSeek", "https://chat.deepseek.com/", "AI"),
    entry(
        "perplexity",
        "Perplexity",
        "https://www.perplexity.ai/",
        "AI",
    ),
    entry("grok", "Grok", "https://grok.com/", "AI"),
    entry(
        "mistral",
        "Le Chat (Mistral)",
        "https://chat.mistral.ai/",
        "AI",
    ),
];

/// Categories in display order (first appearance in [`CATALOG`]).
pub fn categories() -> Vec<&'static str> {
    let mut ordered: Vec<&'static str> = Vec::new();
    for entry in CATALOG {
        if !ordered.contains(&entry.category) {
            ordered.push(entry.category);
        }
    }
    ordered
}

/// Recommended background behavior for a newly-added catalog service.
/// Custom URLs intentionally default to off.
pub fn background_by_default(url: &str) -> bool {
    CATALOG
        .iter()
        .find(|entry| entry.url == url)
        .is_some_and(|entry| entry.background_by_default)
}

const fn entry(
    key: &'static str,
    name: &'static str,
    url: &'static str,
    category: &'static str,
) -> Entry {
    Entry {
        key,
        name,
        url,
        category,
        background_by_default: false,
    }
}

const fn background_entry(
    key: &'static str,
    name: &'static str,
    url: &'static str,
    category: &'static str,
) -> Entry {
    Entry {
        key,
        name,
        url,
        category,
        background_by_default: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{background_by_default, categories, CATALOG};

    #[test]
    fn catalog_only_contains_supported_service_categories() {
        assert_eq!(
            categories(),
            ["Messaging", "Email", "Calendar", "Tasks", "AI"]
        );
        assert!(CATALOG.iter().all(|entry| matches!(
            entry.category,
            "Messaging" | "Email" | "Calendar" | "Tasks" | "AI"
        )));
    }

    #[test]
    fn catalog_excludes_video_conference_only_services() {
        let excluded = ["gmeet", "zoom", "whereby"];

        assert!(excluded
            .iter()
            .all(|key| CATALOG.iter().all(|entry| entry.key != *key)));
    }

    #[test]
    fn only_focused_messengers_run_in_background_by_default() {
        let enabled = [
            "whatsapp",
            "telegram",
            "messenger",
            "element",
            "gmessages",
            "threema",
            "groupme",
        ];

        assert!(CATALOG
            .iter()
            .all(|entry| entry.background_by_default == enabled.contains(&entry.key)));
        assert!(background_by_default("https://web.whatsapp.com/"));
        assert!(!background_by_default("https://teams.microsoft.com/"));
        assert!(!background_by_default("https://custom.example/"));
    }
}
