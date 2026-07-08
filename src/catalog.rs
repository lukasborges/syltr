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
}

pub const CATALOG: &[Entry] = &[
    // Messaging
    Entry { key: "whatsapp",  name: "WhatsApp Web",     url: "https://web.whatsapp.com/" },
    Entry { key: "telegram",  name: "Telegram Web",     url: "https://web.telegram.org/" },
    Entry { key: "messenger", name: "Messenger",        url: "https://www.messenger.com/" },
    Entry { key: "slack",     name: "Slack",            url: "https://app.slack.com/client" },
    Entry { key: "discord",   name: "Discord",          url: "https://discord.com/app" },
    Entry { key: "element",   name: "Element (Matrix)", url: "https://app.element.io/" },
    Entry { key: "skype",     name: "Skype",            url: "https://web.skype.com/" },
    Entry { key: "gmessages", name: "Google Messages",  url: "https://messages.google.com/web/" },
    Entry { key: "threema",   name: "Threema Web",      url: "https://web.threema.ch/" },
    Entry { key: "groupme",   name: "GroupMe",          url: "https://web.groupme.com/" },
    // Social
    Entry { key: "instagram", name: "Instagram",        url: "https://www.instagram.com/direct/inbox/" },
    Entry { key: "linkedin",  name: "LinkedIn",         url: "https://www.linkedin.com/messaging/" },
    Entry { key: "x",         name: "X",                url: "https://x.com/messages" },
    Entry { key: "reddit",    name: "Reddit",           url: "https://www.reddit.com/" },
    Entry { key: "twitch",    name: "Twitch",           url: "https://www.twitch.tv/" },
    // Google
    Entry { key: "gmail",     name: "Gmail",            url: "https://mail.google.com/" },
    Entry { key: "gcalendar", name: "Google Calendar",  url: "https://calendar.google.com/" },
    Entry { key: "gchat",     name: "Google Chat",      url: "https://chat.google.com/" },
    Entry { key: "gmeet",     name: "Google Meet",      url: "https://meet.google.com/" },
    Entry { key: "gdrive",    name: "Google Drive",     url: "https://drive.google.com/" },
    Entry { key: "gdocs",     name: "Google Docs",      url: "https://docs.google.com/" },
    Entry { key: "gvoice",    name: "Google Voice",     url: "https://voice.google.com/" },
    Entry { key: "gphotos",   name: "Google Photos",    url: "https://photos.google.com/" },
    // Microsoft
    Entry { key: "teams",     name: "Microsoft Teams",  url: "https://teams.microsoft.com/" },
    Entry { key: "outlook",   name: "Outlook",          url: "https://outlook.live.com/mail/" },
    Entry { key: "onedrive",  name: "OneDrive",         url: "https://onedrive.live.com/" },
    Entry { key: "mstodo",    name: "Microsoft To Do",  url: "https://to-do.office.com/" },
    // Email
    Entry { key: "proton",    name: "Proton Mail",      url: "https://mail.proton.me/" },
    Entry { key: "tuta",      name: "Tuta (Tutanota)",  url: "https://app.tuta.com/" },
    Entry { key: "fastmail",  name: "Fastmail",         url: "https://app.fastmail.com/" },
    Entry { key: "zohomail",  name: "Zoho Mail",        url: "https://mail.zoho.com/" },
    Entry { key: "yahoomail", name: "Yahoo Mail",       url: "https://mail.yahoo.com/" },
    Entry { key: "icloud",    name: "iCloud Mail",      url: "https://www.icloud.com/mail" },
    // Productivity
    Entry { key: "notion",    name: "Notion",           url: "https://www.notion.so/" },
    Entry { key: "trello",    name: "Trello",           url: "https://trello.com/" },
    Entry { key: "todoist",   name: "Todoist",          url: "https://app.todoist.com/" },
    Entry { key: "asana",     name: "Asana",            url: "https://app.asana.com/" },
    Entry { key: "clickup",   name: "ClickUp",          url: "https://app.clickup.com/" },
    Entry { key: "linear",    name: "Linear",           url: "https://linear.app/" },
    // AI
    Entry { key: "chatgpt",    name: "ChatGPT",           url: "https://chatgpt.com/" },
    Entry { key: "claude",     name: "Claude",            url: "https://claude.ai/" },
    Entry { key: "gemini",     name: "Gemini",            url: "https://gemini.google.com/" },
    Entry { key: "copilot",    name: "Microsoft Copilot", url: "https://copilot.microsoft.com/" },
    Entry { key: "deepseek",   name: "DeepSeek",          url: "https://chat.deepseek.com/" },
    Entry { key: "perplexity", name: "Perplexity",        url: "https://www.perplexity.ai/" },
    Entry { key: "grok",       name: "Grok",              url: "https://grok.com/" },
    Entry { key: "mistral",    name: "Le Chat (Mistral)", url: "https://chat.mistral.ai/" },
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
