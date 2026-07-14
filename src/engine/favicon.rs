//! Favicon wiring for the rail icon: WebKit's native favicon tracking as the
//! fast path, plus the JS canvas rasterization (see `scripts::FAVICON_JS`) as
//! the robust path — it handles SVG-only sites the native tracker misses.
//!
//! The texture is stored in a shared cell and observers are notified, so the
//! rail's grouped icon can pick it up (several instances share one icon).

use std::cell::RefCell;
use std::rc::Rc;

use base64::Engine;
use gtk::gdk;
use webkit6::prelude::*;

pub(super) fn wire(
    webview: &webkit6::WebView,
    ucm: &webkit6::UserContentManager,
    store: Rc<RefCell<Option<gdk::Texture>>>,
    notify: Rc<dyn Fn()>,
) {
    let set: Rc<dyn Fn(Option<gdk::Texture>)> = Rc::new(move |texture| {
        *store.borrow_mut() = texture;
        notify();
    });

    // Fast path: the raster favicon WebKit already tracks.
    set(webview.favicon());
    {
        let set = set.clone();
        webview.connect_favicon_notify(move |wv| set(wv.favicon()));
    }
    // Robust path: the page's real icon rasterized to PNG by the injected JS.
    {
        let set = set.clone();
        ucm.connect_script_message_received(Some("faviconReady"), move |_, value| {
            if let Some(texture) = png_data_url_to_texture(&value.to_str()) {
                set(Some(texture));
            }
        });
    }
}

/// Converts a "data:image/png;base64,…" data URL into a texture.
fn png_data_url_to_texture(data_url: &str) -> Option<gdk::Texture> {
    let b64 = data_url.strip_prefix("data:image/png;base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&bytes)).ok()
}
