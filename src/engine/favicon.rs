//! Favicon wiring for the rail icon: WebKit's native favicon tracking as the
//! fast path, plus the JS canvas rasterization (see `scripts::FAVICON_JS`) as
//! the robust path — it handles SVG-only sites the native tracker misses.

use base64::Engine;
use gtk::gdk;
use webkit6::prelude::*;

use crate::icon::ServiceIcon;

pub(super) fn wire(
    webview: &webkit6::WebView,
    ucm: &webkit6::UserContentManager,
    icon: &ServiceIcon,
) {
    // Fast path: the raster favicon WebKit already tracks.
    icon.set_favicon(webview.favicon().as_ref());
    {
        let icon = icon.clone();
        webview.connect_favicon_notify(move |wv| {
            icon.set_favicon(wv.favicon().as_ref());
        });
    }
    // Robust path: the page's real icon rasterized to PNG by the injected JS.
    {
        let icon = icon.clone();
        ucm.connect_script_message_received(Some("faviconReady"), move |_, value| {
            if let Some(texture) = png_data_url_to_texture(&value.to_str()) {
                icon.set_favicon(Some(&texture));
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
