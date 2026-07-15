//! Syltr — an all-in-one messaging service aggregator for GNOME.
//!
//! Stack: GTK4 + libadwaita + WebKitGTK 6, in Rust. The web engine is
//! isolated in the `engine` module; the rest of the app never touches
//! `webkit6` directly.

mod catalog;
mod config;
mod engine;
mod icon;
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

/* Stacked-cards hint behind the icon of a service with 2+ instances. */
.instance-stack {
    border-radius: 11px;
    box-shadow: 3px -3px 0 alpha(@window_fg_color, 0.18);
}

/* A disabled service: dimmed tile in the rail. */
.service-disabled {
    opacity: 0.4;
}

/* Do-not-disturb toggle: red icon while active. */
.dnd-toggle:checked {
    color: #e01b24;
}

/* The active instance in the chooser popover. */
.instance-current {
    font-weight: bold;
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
    init_i18n();
    register_resources();

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| load_css());
    app.connect_activate(window::build);

    app.run()
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

/// Registers the embedded GResource that holds the bundled service icons.
fn register_resources() {
    let bytes =
        gtk::glib::Bytes::from_static(include_bytes!(concat!(env!("OUT_DIR"), "/syltr.gresource")));
    match gtk::gio::Resource::from_data(&bytes) {
        Ok(resource) => gtk::gio::resources_register(&resource),
        Err(e) => eprintln!("syltr: failed to register icon resources: {e}"),
    }
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
