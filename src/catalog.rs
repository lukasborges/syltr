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
}

pub const CATALOG: &[Entry] = &[
    entry(
        "whatsapp",
        "WhatsApp Web",
        "https://web.whatsapp.com/",
        "Messaging",
    ),
    entry(
        "telegram",
        "Telegram Web",
        "https://web.telegram.org/",
        "Messaging",
    ),
    entry(
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
    entry(
        "element",
        "Element (Matrix)",
        "https://app.element.io/",
        "Messaging",
    ),
    entry("skype", "Skype", "https://web.skype.com/", "Messaging"),
    entry(
        "gmessages",
        "Google Messages",
        "https://messages.google.com/web/",
        "Messaging",
    ),
    entry(
        "threema",
        "Threema Web",
        "https://web.threema.ch/",
        "Messaging",
    ),
    entry(
        "groupme",
        "GroupMe",
        "https://web.groupme.com/",
        "Messaging",
    ),
    entry(
        "instagram",
        "Instagram",
        "https://www.instagram.com/direct/inbox/",
        "Social",
    ),
    entry(
        "linkedin",
        "LinkedIn",
        "https://www.linkedin.com/messaging/",
        "Social",
    ),
    entry("x", "X", "https://x.com/messages", "Social"),
    entry("reddit", "Reddit", "https://www.reddit.com/", "Social"),
    entry("twitch", "Twitch", "https://www.twitch.tv/", "Social"),
    entry("gmail", "Gmail", "https://mail.google.com/", "Google"),
    entry(
        "gcalendar",
        "Google Calendar",
        "https://calendar.google.com/",
        "Google",
    ),
    entry("gchat", "Google Chat", "https://chat.google.com/", "Google"),
    entry("gmeet", "Google Meet", "https://meet.google.com/", "Google"),
    entry(
        "gdrive",
        "Google Drive",
        "https://drive.google.com/",
        "Google",
    ),
    entry("gdocs", "Google Docs", "https://docs.google.com/", "Google"),
    entry(
        "gvoice",
        "Google Voice",
        "https://voice.google.com/",
        "Google",
    ),
    entry(
        "gphotos",
        "Google Photos",
        "https://photos.google.com/",
        "Google",
    ),
    entry(
        "teams",
        "Microsoft Teams",
        "https://teams.microsoft.com/",
        "Microsoft",
    ),
    entry(
        "outlook",
        "Outlook",
        "https://outlook.live.com/mail/",
        "Microsoft",
    ),
    entry(
        "onedrive",
        "OneDrive",
        "https://onedrive.live.com/",
        "Microsoft",
    ),
    entry(
        "mstodo",
        "Microsoft To Do",
        "https://to-do.office.com/",
        "Microsoft",
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
    entry("notion", "Notion", "https://www.notion.so/", "Productivity"),
    entry("trello", "Trello", "https://trello.com/", "Productivity"),
    entry(
        "todoist",
        "Todoist",
        "https://app.todoist.com/",
        "Productivity",
    ),
    entry("asana", "Asana", "https://app.asana.com/", "Productivity"),
    entry(
        "clickup",
        "ClickUp",
        "https://app.clickup.com/",
        "Productivity",
    ),
    entry("linear", "Linear", "https://linear.app/", "Productivity"),
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

/// Services the app starts with on first run.
pub const DEFAULT_KEYS: &[&str] = &[
    // Messaging
    "whatsapp",
    "telegram",
    // Google
    "gmail",
    "gcalendar",
    "gchat",
    "gmeet",
    // Microsoft
    "teams",
    // AI
    "chatgpt",
    "deepseek",
    "perplexity",
];

pub fn find(key: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.key == key)
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
    }
}
