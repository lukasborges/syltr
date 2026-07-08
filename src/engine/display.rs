//! Display handler: cursor changes, unread badge from the title, and favicon
//! download into the service icon.

use std::rc::Rc;

use cef::*;
use gtk::prelude::*;
use gtk::{gdk, glib};

use super::render::RenderState;
use crate::icon::ServiceIcon;

wrap_display_handler! {
    pub(crate) struct DisplayHandlerBuilder {
        state: Rc<RenderState>,
        icon: ServiceIcon,
    }

    impl DisplayHandler {
        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: ::std::os::raw::c_ulong,
            type_: CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> ::std::os::raw::c_int {
            self.state.area().set_cursor_from_name(Some(cursor_name(type_)));
            1
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            if let Some(title) = title {
                self.icon.set_badge(unread_from_title(&title.to_string()));
            }
        }

        fn on_favicon_urlchange(
            &self,
            browser: Option<&mut Browser>,
            icon_urls: Option<&mut CefStringList>,
        ) {
            let (Some(browser), Some(urls)) = (browser, icon_urls) else { return };
            let Some(host) = browser.host() else { return };
            let list = std::mem::take(urls);
            let Some(url) = list.into_iter().next() else { return };
            let mut callback = FaviconCallbackBuilder::build(self.icon.clone());
            host.download_image(
                Some(&CefString::from(url.as_str())),
                1,  // is_favicon
                64, // max_image_size
                0,  // bypass_cache
                Some(&mut callback),
            );
        }
    }
}

impl DisplayHandlerBuilder {
    pub(crate) fn build(state: Rc<RenderState>, icon: ServiceIcon) -> DisplayHandler {
        Self::new(state, icon)
    }
}

wrap_download_image_callback! {
    struct FaviconCallbackBuilder {
        icon: ServiceIcon,
    }

    impl DownloadImageCallback {
        fn on_download_image_finished(
            &self,
            _image_url: Option<&CefString>,
            _http_status_code: ::std::os::raw::c_int,
            image: Option<&mut cef::Image>,
        ) {
            let Some(image) = image else { return };
            let mut pw = 0;
            let mut ph = 0;
            let Some(png) = image.as_png(1.0, 1, Some(&mut pw), Some(&mut ph)) else {
                return;
            };
            let size = png.size();
            if size == 0 {
                return;
            }
            let bytes = unsafe { std::slice::from_raw_parts(png.raw_data() as *const u8, size) };
            if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from(bytes)) {
                self.icon.set_favicon(Some(&texture));
            }
        }
    }
}

impl FaviconCallbackBuilder {
    fn build(icon: ServiceIcon) -> DownloadImageCallback {
        Self::new(icon)
    }
}

/// Extracts the unread count from a title (e.g. "(5) WhatsApp", "Inbox (12) …",
/// "5 messages"). Returns 0 when none is found.
fn unread_from_title(title: &str) -> u32 {
    let bytes = title.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if matches!(c, b'(' | b'[' | b'{') {
            if let Some(n) = leading_number(&bytes[i + 1..]) {
                return n;
            }
        }
    }
    leading_number(bytes).unwrap_or(0)
}

/// Parses the run of ASCII digits at the start of `bytes`, if any.
fn leading_number(bytes: &[u8]) -> Option<u32> {
    let mut n = 0u32;
    let mut found = false;
    for &d in bytes {
        if !d.is_ascii_digit() {
            break;
        }
        n = n.saturating_mul(10).saturating_add((d - b'0') as u32);
        found = true;
    }
    found.then_some(n)
}

fn cursor_name(t: CursorType) -> &'static str {
    if t == CursorType::HAND {
        "pointer"
    } else if t == CursorType::IBEAM {
        "text"
    } else if t == CursorType::CROSS {
        "crosshair"
    } else if t == CursorType::WAIT {
        "wait"
    } else if t == CursorType::HELP {
        "help"
    } else if t == CursorType::MOVE {
        "move"
    } else if t == CursorType::PROGRESS {
        "progress"
    } else if t == CursorType::NOTALLOWED {
        "not-allowed"
    } else if t == CursorType::NODROP {
        "no-drop"
    } else if t == CursorType::COPY {
        "copy"
    } else if t == CursorType::CONTEXTMENU {
        "context-menu"
    } else if t == CursorType::COLUMNRESIZE {
        "col-resize"
    } else if t == CursorType::ROWRESIZE {
        "row-resize"
    } else if t == CursorType::EASTWESTRESIZE {
        "ew-resize"
    } else if t == CursorType::NORTHSOUTHRESIZE {
        "ns-resize"
    } else if t == CursorType::ZOOMIN {
        "zoom-in"
    } else if t == CursorType::NONE {
        "none"
    } else {
        "default"
    }
}
