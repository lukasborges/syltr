//! Discovery of spell-check dictionaries installed on the system.
//!
//! Chromium ships its own spell checker (downloading `.bdic` dictionaries), but
//! we offer the user the languages already installed as hunspell/myspell so the
//! default selection matches what the system provides.

/// Directories where hunspell/myspell dictionaries are installed.
const DICTIONARY_DIRS: &[&str] = &[
    "/usr/share/hunspell",
    "/usr/share/myspell/dicts",
    "/usr/local/share/hunspell",
];

/// Environment variables consulted, in order, to detect the user's languages.
const LOCALE_VARS: &[&str] = &["LC_MESSAGES", "LANG", "LANGUAGE"];

/// Language codes of the hunspell/myspell dictionaries installed on the system.
pub fn available_dictionaries() -> Vec<String> {
    let mut languages: Vec<String> = Vec::new();
    for dir in DICTIONARY_DIRS {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("dic") {
                continue;
            }
            if let Some(lang) = path.file_stem().and_then(|s| s.to_str()) {
                if !languages.iter().any(|l| l == lang) {
                    languages.push(lang.to_string());
                }
            }
        }
    }
    languages
}

/// Installed dictionaries, ordered with the locale's languages first.
pub fn default_languages() -> Vec<String> {
    let available = available_dictionaries();
    let mut ordered: Vec<String> = Vec::new();
    for lang in locale_languages() {
        if available.contains(&lang) && !ordered.contains(&lang) {
            ordered.push(lang);
        }
    }
    for lang in available {
        if !ordered.contains(&lang) {
            ordered.push(lang);
        }
    }
    ordered
}

/// Language codes derived from the locale environment variables (e.g. "pt_BR").
fn locale_languages() -> Vec<String> {
    let mut languages: Vec<String> = Vec::new();
    for var in LOCALE_VARS {
        let Ok(value) = std::env::var(var) else {
            continue;
        };
        for part in value.split(':') {
            let lang = strip_locale_suffixes(part);
            if is_real_language(lang) && !languages.iter().any(|l| l == lang) {
                languages.push(lang.to_string());
            }
        }
    }
    languages
}

/// Strips the encoding (`.UTF-8`) and modifier (`@euro`) suffixes from a locale.
fn strip_locale_suffixes(locale: &str) -> &str {
    locale
        .split('.')
        .next()
        .unwrap_or("")
        .split('@')
        .next()
        .unwrap_or("")
}

/// Whether a locale token names an actual language (not empty, C or POSIX).
fn is_real_language(lang: &str) -> bool {
    !lang.is_empty() && lang != "C" && lang != "POSIX"
}
