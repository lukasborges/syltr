//! Web engine layer — CEF (Chromium) in offscreen (OSR) mode.
//!
//! The rest of the app only uses [`ServiceView`]; everything CEF-specific
//! (multiprocess, message loop, OSR→Cairo, input) is contained in this module's
//! submodules. Each service is a windowless CEF browser rendering into a
//! `GtkDrawingArea`, with an isolated session/cache per service.

mod bootstrap;
mod browser_slot;
mod client;
mod context_menu;
mod display;
mod download;
mod lifespan;
mod permission;
mod prefs;
mod render;
mod request_context;

pub use bootstrap::{init_cef, shutdown_cef, start_pump};
pub use browser_slot::BrowserSlot;

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use cef::*;
use gtk::prelude::*;

use client::ClientBuilder;
use prefs::apply_spell_prefs;
use render::{draw, RenderState};
use request_context::RequestContextHandlerBuilder;

use crate::icon::ServiceIcon;
use crate::input;

/// White opaque background (ARGB); pages without their own background would be
/// black in OSR otherwise.
const OPAQUE_WHITE: u32 = 0xFFFF_FFFF;
const FRAME_RATE: i32 = 60;

/// A single service's web view: its drawing area, browser handle and icon.
#[derive(Clone)]
pub struct ServiceView {
    root: gtk::Widget,
    slot: Rc<BrowserSlot>,
    icon: ServiceIcon,
    context: Option<RequestContext>,
    home: String,
}

impl ServiceView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _id: &str,
        name: &str,
        url: &str,
        session_dir: &Path,
        _app: &adw::Application,
        _dnd: Rc<Cell<bool>>,
        muted: bool,
        spell_langs: &[String],
        _media_enabled: bool,
    ) -> Self {
        let area = gtk::DrawingArea::builder().hexpand(true).vexpand(true).build();
        let state = RenderState::new(area.clone());
        {
            let state = state.clone();
            area.set_draw_func(move |_, cr, _w, _h| draw(&state, cr));
        }

        let context = create_request_context(session_dir);
        let icon = ServiceIcon::new(name);
        let slot = BrowserSlot::new();

        // Creation is ASYNCHRONOUS: the Chrome runtime builds the per-service
        // profile in the background, so the browser arrives in on_after_created
        // (stored in `slot`). The initial notification/spell state is applied
        // there too, once the context's profile is ready.
        let window_info = WindowInfo {
            windowless_rendering_enabled: 1,
            ..Default::default()
        };
        let browser_settings = BrowserSettings {
            windowless_frame_rate: FRAME_RATE,
            background_color: OPAQUE_WHITE,
            ..Default::default()
        };
        let mut context_arg = context.clone();
        browser_host_create_browser(
            Some(&window_info),
            Some(&mut ClientBuilder::build(
                state.clone(),
                slot.clone(),
                icon.clone(),
                muted,
                spell_langs.to_vec(),
                context.clone(),
            )),
            Some(&CefString::from(url)),
            Some(&browser_settings),
            None,
            context_arg.as_mut(),
        );

        {
            let state = state.clone();
            let slot = slot.clone();
            area.connect_resize(move |area, w, h| {
                let scale = area.scale_factor().max(1) as f32;
                state.set_size(w.max(1), h.max(1), scale);
                if let Some(host) = slot.host() {
                    host.was_resized();
                }
            });
        }

        input::attach(&area, slot.clone());

        Self {
            root: area.upcast(),
            slot,
            icon,
            context,
            home: url.to_string(),
        }
    }

    /// Toggles the service's notifications at the context's content-setting level.
    pub fn set_notifications_enabled(&self, enabled: bool) {
        if let Some(ctx) = &self.context {
            let value = if enabled {
                ContentSettingValues::ALLOW
            } else {
                ContentSettingValues::BLOCK
            };
            ctx.set_content_setting(None, None, ContentSettingTypes::NOTIFICATIONS, value);
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }

    pub fn icon(&self) -> &gtk::Widget {
        self.icon.widget()
    }

    pub fn reload(&self) {
        if let Some(browser) = self.slot.browser() {
            browser.reload();
        }
    }

    pub fn go_home(&self) {
        if let Some(frame) = self.slot.main_frame() {
            frame.load_url(Some(&CefString::from(self.home.as_str())));
        }
    }

    pub fn set_spell_languages(&self, langs: &[String]) {
        if let Some(ctx) = &self.context {
            apply_spell_prefs(ctx, langs);
        }
    }

    /// Media/calls work natively in Chromium, so this is a no-op.
    pub fn set_media_enabled(&self, _enabled: bool) {}
}

/// Creates a request context with an isolated session/cache under `session_dir`
/// (a subdirectory of root_cache_path, as the Chrome runtime requires).
fn create_request_context(session_dir: &Path) -> Option<RequestContext> {
    let _ = std::fs::create_dir_all(session_dir);
    let settings = RequestContextSettings {
        cache_path: CefString::from(session_dir.to_str().unwrap_or_default()),
        ..Default::default()
    };
    request_context_create_context(
        Some(&settings),
        Some(&mut RequestContextHandlerBuilder::build()),
    )
}
