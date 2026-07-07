//! Catálogo de serviços conhecidos ("recipes", no linguajar do Franz).
//!
//! É só uma tabela estática usada pelo diálogo "Adicionar serviço". Qualquer
//! URL personalizada também é aceita — o catálogo apenas dá atalhos.

pub struct Entry {
    /// chave estável usada como base do id da instância
    pub key: &'static str,
    pub name: &'static str,
    pub url: &'static str,
}

pub const CATALOG: &[Entry] = &[
    // Mensagens
    Entry { key: "whatsapp",  name: "WhatsApp Web",   url: "https://web.whatsapp.com/" },
    Entry { key: "telegram",  name: "Telegram Web",   url: "https://web.telegram.org/" },
    // Google
    Entry { key: "gmail",     name: "Gmail",          url: "https://mail.google.com/" },
    Entry { key: "gcalendar", name: "Google Agenda",  url: "https://calendar.google.com/" },
    Entry { key: "gchat",     name: "Google Chat",    url: "https://chat.google.com/" },
    Entry { key: "gmeet",     name: "Google Meet",    url: "https://meet.google.com/" },
    // Microsoft
    Entry { key: "teams",     name: "Microsoft Teams", url: "https://teams.microsoft.com/" },
    // IA
    Entry { key: "chatgpt",    name: "ChatGPT",       url: "https://chatgpt.com/" },
    Entry { key: "deepseek",   name: "DeepSeek",      url: "https://chat.deepseek.com/" },
    Entry { key: "perplexity", name: "Perplexity",    url: "https://www.perplexity.ai/" },
    // Extras (não entram por padrão, mas ficam disponíveis para adicionar)
    Entry { key: "slack",     name: "Slack",          url: "https://app.slack.com/client" },
    Entry { key: "discord",   name: "Discord",        url: "https://discord.com/app" },
    Entry { key: "messenger", name: "Messenger",      url: "https://www.messenger.com/" },
];

/// Serviços com que o app inicia na primeira execução.
pub const DEFAULT_KEYS: &[&str] = &[
    // Mensagens
    "whatsapp", "telegram",
    // Google
    "gmail", "gcalendar", "gchat", "gmeet",
    // Microsoft
    "teams",
    // IA
    "chatgpt", "deepseek", "perplexity",
];

pub fn find(key: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.key == key)
}
