//! Catalog of known services ("recipes").
//!
//! It is just a static table used by the "Add service" dialog. Any custom URL
//! is accepted too — the catalog only provides shortcuts.

pub struct Entry {
    /// Stable key used as the base of the instance id.
    pub key: &'static str,
    pub name: &'static str,
    pub url: &'static str,
}

pub const CATALOG: &[Entry] = &[
    // Messaging
    Entry { key: "whatsapp",  name: "WhatsApp Web",   url: "https://web.whatsapp.com/" },
    Entry { key: "telegram",  name: "Telegram Web",   url: "https://web.telegram.org/" },
    // Google
    Entry { key: "gmail",     name: "Gmail",          url: "https://mail.google.com/" },
    Entry { key: "gcalendar", name: "Google Calendar", url: "https://calendar.google.com/" },
    Entry { key: "gchat",     name: "Google Chat",    url: "https://chat.google.com/" },
    Entry { key: "gmeet",     name: "Google Meet",    url: "https://meet.google.com/" },
    // Microsoft
    Entry { key: "teams",     name: "Microsoft Teams", url: "https://teams.microsoft.com/" },
    // AI
    Entry { key: "chatgpt",    name: "ChatGPT",       url: "https://chatgpt.com/" },
    Entry { key: "deepseek",   name: "DeepSeek",      url: "https://chat.deepseek.com/" },
    Entry { key: "perplexity", name: "Perplexity",    url: "https://www.perplexity.ai/" },
    // Extras (not enabled by default, but available to add)
    Entry { key: "slack",     name: "Slack",          url: "https://app.slack.com/client" },
    Entry { key: "discord",   name: "Discord",        url: "https://discord.com/app" },
    Entry { key: "messenger", name: "Messenger",      url: "https://www.messenger.com/" },
];

/// Services the app starts with on first run.
pub const DEFAULT_KEYS: &[&str] = &[
    // Messaging
    "whatsapp", "telegram",
    // Google
    "gmail", "gcalendar", "gchat", "gmeet",
    // Microsoft
    "teams",
    // AI
    "chatgpt", "deepseek", "perplexity",
];

pub fn find(key: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.key == key)
}
