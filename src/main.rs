//! Syltr — a Franz-style messaging service aggregator for GNOME.
//!
//! Stack: GTK4 + libadwaita + CEF (Chromium), in Rust. The web engine is
//! isolated in `engine.rs`, rendered offscreen (OSR) into GTK drawing areas.

mod catalog;
mod config;
mod engine;
mod icon;
mod imgproxy;
mod input;
mod spellcheck;
mod window;

use adw::prelude::*;
use gtk::glib;

pub const APP_ID: &str = "dev.syltr.Syltr";

const STYLE: &str = "
.service-initial {
    font-weight: bold;
    font-size: 15px;
}

/* Unread badge (top-right corner of the icon). */
.unread-badge {
    background-color: #e01b24;
    color: #ffffff;
    font-size: 10px;
    font-weight: bold;
    padding: 0 4px;
    margin: -2px -2px 0 0;
    min-width: 10px;
    border-radius: 999px;
}

/* Active item: a full-width highlight across the rail (no corners), with an
   accent stroke flush against the window's left edge (full height). */
.rail {
    padding: 0;
}
.rail row {
    margin: 0;
    border-radius: 0;
}
.rail row:hover {
    background-color: alpha(@window_fg_color, 0.05);
}
.rail row:selected,
.rail row:selected:hover {
    background-image: none;
    /* Very subtle highlight: the accent stroke is the primary indicator
       (otherwise a light background bleeds through the favicon's transparent
       areas and washes it out). */
    background-color: alpha(@window_fg_color, 0.04);
    box-shadow: inset 3px 0 0 @accent_bg_color;
}
";

fn main() -> glib::ExitCode {
    // Bootstrap CEF BEFORE GTK: in a subprocess this exits immediately.
    if !engine::init_cef() {
        return glib::ExitCode::SUCCESS;
    }

    init_i18n();

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_startup(|_| load_css());
    app.connect_activate(window::build);

    let code = app.run();
    engine::shutdown_cef();
    code
}

/// Sets up interface translation according to the system language.
/// Source strings are in English; translations live in <data>/locale.
fn init_i18n() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    // The package installs to /usr/share/locale; SYLTR_LOCALE_DIR overrides it.
    let locale_dir = std::env::var_os("SYLTR_LOCALE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/share/locale"));
    let _ = gettextrs::bindtextdomain("syltr", locale_dir);
    let _ = gettextrs::bind_textdomain_codeset("syltr", "UTF-8");
    let _ = gettextrs::textdomain("syltr");
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
