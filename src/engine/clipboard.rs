//! GTK clipboard fallback for images that WebKit omits from paste events.

use std::cell::Cell;
use std::rc::Rc;

use base64::Engine;
use gtk::gdk;
use gtk::prelude::*;

use super::scripts::run_js;

const MAX_PNG_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn wire(webview: &webkit6::WebView, ucm: &webkit6::UserContentManager) {
    let pending = Rc::new(Cell::new(false));
    let webview = webview.clone();
    ucm.connect_script_message_received(Some("syltrPasteImage"), move |_, _| {
        if pending.replace(true) {
            return;
        }
        let Some(display) = gdk::Display::default() else {
            pending.set(false);
            return;
        };
        let pending = pending.clone();
        let webview = webview.clone();
        display
            .clipboard()
            .read_texture_async(None::<&gtk::gio::Cancellable>, move |result| {
                pending.set(false);
                let Ok(Some(texture)) = result else { return };
                let png = texture.save_to_png_bytes();
                if png.len() > MAX_PNG_BYTES {
                    eprintln!("[syltr] clipboard image is too large to paste");
                    return;
                }
                let encoded = base64::engine::general_purpose::STANDARD.encode(png.as_ref());
                run_js(&webview, &format!("window.__syltrPasteImage({encoded:?});"));
            });
    });
}
