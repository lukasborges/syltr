//! Syltr — agregador de serviços de mensagens (estilo Franz) para o GNOME.
//!
//! Stack: GTK4 + libadwaita + WebKitGTK 6, em Rust. A engine web fica isolada
//! em `engine.rs` (hoje WebKit, migração futura para Chromium/CEF).

mod catalog;
mod config;
mod engine;
mod icon;
mod input;
mod window;

use adw::prelude::*;
use gtk::glib;

pub const APP_ID: &str = "dev.syltr.Syltr";

const STYLE: &str = "
.service-initial {
    font-weight: bold;
    font-size: 15px;
}

/* Badge de não lidas (canto superior direito do ícone). */
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

/* Item ativo: realce ocupando a largura toda do rail (sem cantos), com um
   traço de acento colado na borda esquerda da janela (altura cheia). */
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
    /* realce bem sutil: o traço de acento é o indicador principal (senão o
       fundo claro vaza pelas áreas transparentes do favicon e o lava). */
    background-color: alpha(@window_fg_color, 0.04);
    box-shadow: inset 3px 0 0 @accent_bg_color;
}
";

fn main() -> glib::ExitCode {
    // Bootstrap do CEF ANTES de GTK: no subprocesso, sai imediatamente.
    if !engine::init_cef() {
        return glib::ExitCode::SUCCESS;
    }

    init_i18n();

    // Inicializa GTK + libadwaita antes de qualquer widget.
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_startup(|_| load_css());
    app.connect_activate(window::build);

    let code = app.run();
    engine::shutdown_cef();
    code
}

/// Configura a tradução da interface conforme o idioma do sistema.
/// As strings-fonte estão em inglês; traduções ficam em <data>/locale.
fn init_i18n() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    // Pacote instala em /usr/share/locale; permite sobrepor via SYLTR_LOCALE_DIR.
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
