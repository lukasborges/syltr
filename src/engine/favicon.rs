//! Favicon wiring for the rail icon: WebKit's native favicon tracking as the
//! fast path, plus the JS canvas rasterization (see `scripts::FAVICON_JS`) as
//! the robust path — it handles SVG-only sites the native tracker misses.
//!
//! The texture is stored in a shared cell and observers are notified, so the
//! rail's grouped icon can pick it up (several instances share one icon).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use base64::Engine;
use gtk::gdk;
use gtk::prelude::*;
use webkit6::prelude::*;

const CACHE_FILE: &str = "favicon.png";

pub(super) fn load_cached(session_dir: &Path) -> Option<gdk::Texture> {
    gdk::Texture::from_filename(session_dir.join(CACHE_FILE)).ok()
}

pub(super) fn wire(
    webview: &webkit6::WebView,
    ucm: &webkit6::UserContentManager,
    store: Rc<RefCell<Option<gdk::Texture>>>,
    notify: Rc<dyn Fn()>,
    session_dir: &Path,
) {
    let cache_file = session_dir.join(CACHE_FILE);
    let set: Rc<dyn Fn(Option<gdk::Texture>)> = Rc::new(move |texture| {
        // A transient `None` during navigation must not erase the last real
        // favicon; that cached icon is what suspended/disabled services show.
        if let Some(texture) = texture {
            save_cached(&texture, &cache_file);
            *store.borrow_mut() = Some(texture);
            notify();
        }
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

fn save_cached(texture: &gdk::Texture, path: &PathBuf) {
    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!("syltr: could not create favicon cache directory: {error}");
            return;
        }
    }
    if let Err(error) = texture.save_to_png(path) {
        eprintln!("syltr: could not cache favicon: {error}");
    }
}

/// Converts a "data:image/png;base64,…" data URL into a texture.
fn png_data_url_to_texture(data_url: &str) -> Option<gdk::Texture> {
    let b64 = data_url.strip_prefix("data:image/png;base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&bytes)).ok()
}
