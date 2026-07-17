//! Web engine layer — WebKitGTK 6.
//!
//! The rest of the app only uses [`ServiceView`]; everything WebKit-specific
//! (sessions, permissions, notifications, favicons, downloads) is contained in
//! this module's submodules. Each service is a `WebView` with an isolated
//! network session (own cookies/storage/cache) under its session directory.

mod clipboard;
mod favicon;
mod scripts;
mod session;
#[cfg(test)]
mod tests;
mod unread;
mod user_agent;
mod webapp_scripts;

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use gtk::gdk;
use gtk::prelude::*;
use webkit6::prelude::*;

use scripts::{
    run_js, BACKGROUND_ECONOMY_JS, BLOB_MEDIA_JS, COMPAT_JS, CONSOLE_JS, EMOJI_SPRITE_REPAINT_JS,
    FAVICON_JS, SUPPRESS_MPRIS_JS,
};

/// A callback invoked when a view's favicon or unread count changes.
type ChangeCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// A single service's web view: its widget, WebKit view and the observable
/// state (favicon, unread count) the rail's grouped icon reads.
#[derive(Clone)]
pub struct ServiceView {
    root: gtk::Widget,
    webview: webkit6::WebView,
    /// Whether desktop notifications are forwarded (mute and DND both land here
    /// via [`ServiceView::set_notifications_enabled`]).
    notifications: Rc<Cell<bool>>,
    /// True while the service is loaded for notifications but not selected.
    background_economy: Rc<Cell<bool>>,
    home: String,
    unread: Rc<Cell<u32>>,
    favicon: Rc<RefCell<Option<gdk::Texture>>>,
    on_change: ChangeCallback,
}

impl ServiceView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        name: &str,
        url: &str,
        user_agent: Option<&str>,
        session_dir: &Path,
        app: &adw::Application,
        dnd: Rc<Cell<bool>>,
        muted: bool,
        spell_langs: &[String],
    ) -> Self {
        let network_session = session::build(session_dir);
        session::wire_downloads(&network_session);

        let settings = build_settings(&user_agent::for_service(url, user_agent), url);

        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("faviconReady", None);
        ucm.register_script_message_handler("syltrNotify", None);
        ucm.register_script_message_handler("syltrPasteImage", None);
        // Compatibility shims injected at page start (see COMPAT_JS).
        ucm.add_script(&webkit6::UserScript::new(
            COMPAT_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.add_script(&webkit6::UserScript::new(
            BLOB_MEDIA_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.add_script(&webkit6::UserScript::new(
            scripts::CLIPBOARD_BRIDGE_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.add_script(&webkit6::UserScript::new(
            SUPPRESS_MPRIS_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.add_script(&webkit6::UserScript::new(
            BACKGROUND_ECONOMY_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        ucm.add_script(&webkit6::UserScript::new(
            EMOJI_SPRITE_REPAINT_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
        if debug_enabled() {
            wire_console_capture(&ucm, name);
        }

        if url.contains("teams.microsoft.com") {
            ucm.add_script(&webkit6::UserScript::new(
                webapp_scripts::teams::TEAMS_JS,
                webkit6::UserContentInjectedFrames::AllFrames,
                webkit6::UserScriptInjectionTime::Start,
                &[],
                &[],
            ));
        }

        let policies = webkit6::WebsitePolicies::builder()
            .autoplay(webkit6::AutoplayPolicy::Allow)
            .build();
        let webview = webkit6::WebView::builder()
            .network_session(&network_session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .website_policies(&policies)
            .vexpand(true)
            .hexpand(true)
            .build();

        // A newly visible WebView can retain a blank paint layer for CSS emoji
        // sprites. Run the narrow repaint workaround only after GTK maps it,
        // when WebKit can actually produce a frame.
        webview.connect_map(|wv| {
            scripts::repaint_emoji_sprites(wv);
            wv.queue_draw();
        });

        apply_spell(&webview, spell_langs);
        clipboard::wire(&webview, &ucm);

        // Grant permissions (notifications, media) automatically — this is a
        // dedicated messaging client, so always allowing is the expected
        // behavior.
        webview.connect_permission_request(|_, request| {
            request.allow();
            true
        });

        let notifications = Rc::new(Cell::new(!muted));
        wire_notifications(&ucm, app, id, name, dnd, notifications.clone());

        wire_link_clicks(&webview);

        webview.connect_create(|_wv, action| {
            if let Some(uri) = action.request().and_then(|r| r.uri()) {
                if !uri.is_empty() {
                    open_in_default_browser(&uri);
                }
            }
            None::<gtk::Widget>
        });

        let unread = Rc::new(Cell::new(0u32));
        let background_economy = Rc::new(Cell::new(false));
        let favicon = Rc::new(RefCell::new(favicon::load_cached(session_dir)));
        let on_change: ChangeCallback = Rc::new(RefCell::new(None));
        let notify: Rc<dyn Fn()> = {
            let on_change = on_change.clone();
            Rc::new(move || {
                if let Some(f) = on_change.borrow().as_ref() {
                    f();
                }
            })
        };

        favicon::wire(&webview, &ucm, favicon.clone(), notify.clone(), session_dir);

        // Unread count parsed from the page title; observers (the rail's grouped
        // icon) are notified so they can re-aggregate.
        {
            let unread = unread.clone();
            let notify = notify.clone();
            webview.connect_title_notify(move |wv| {
                let title = wv.title().map(|t| t.to_string()).unwrap_or_default();
                unread.set(unread::from_title(&title));
                notify();
            });
        }

        wire_crash_recovery(&webview);

        // Once a page finishes loading, rasterize the real favicon (SVG
        // included) via JS as a robust fallback to WebKit's own tracking.
        {
            let background_economy = background_economy.clone();
            webview.connect_load_changed(move |wv, event| {
                if event == webkit6::LoadEvent::Committed {
                    scripts::set_background_economy(wv, background_economy.get());
                }
                if event == webkit6::LoadEvent::Finished {
                    run_js(wv, FAVICON_JS);
                    scripts::repaint_emoji_sprites(wv);
                }
            });
        }

        webview.load_uri(url);

        Self {
            root: webview.clone().upcast(),
            webview,
            notifications,
            background_economy,
            home: url.to_string(),
            unread,
            favicon,
            on_change,
        }
    }

    /// Toggles forwarding the service's notifications to the desktop.
    pub fn set_notifications_enabled(&self, enabled: bool) {
        self.notifications.set(enabled);
    }

    /// Marks this view as foreground or background without unloading it. The
    /// hidden view keeps its network connection and JS state, while continuous
    /// visual work is paused until it is selected again.
    pub fn set_active(&self, active: bool) {
        let enabled = !active;
        if self.background_economy.get() == enabled {
            return;
        }
        self.background_economy.set(enabled);
        scripts::set_background_economy(&self.webview, enabled);
        if active {
            self.root.queue_draw();
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }

    /// Current unread count (parsed from the page title).
    pub fn unread(&self) -> u32 {
        self.unread.get()
    }

    /// Current favicon texture, if one has loaded.
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.favicon.borrow().clone()
    }

    /// Registers a callback fired whenever this view's favicon or unread count
    /// changes, so the rail can refresh the grouped icon.
    pub fn set_on_change(&self, f: impl Fn() + 'static) {
        *self.on_change.borrow_mut() = Some(Box::new(f));
    }

    /// Disconnects the rail observer before this view is unloaded.
    pub fn clear_on_change(&self) {
        self.on_change.borrow_mut().take();
    }

    pub fn reload(&self) {
        self.webview.reload();
    }

    /// Navigates back in the service's history, if possible.
    pub fn go_back(&self) {
        if self.webview.can_go_back() {
            self.webview.go_back();
        }
    }

    /// Navigates forward in the service's history, if possible.
    pub fn go_forward(&self) {
        if self.webview.can_go_forward() {
            self.webview.go_forward();
        }
    }

    pub fn go_home(&self) {
        self.webview.load_uri(&self.home);
    }

    /// Opens WebKit's developer tools for this service.
    pub fn show_inspector(&self) {
        if let Some(inspector) = self.webview.inspector() {
            inspector.show();
        }
    }

    pub fn set_spell_languages(&self, langs: &[String]) {
        apply_spell(&self.webview, langs);
    }

    /// Applies a new user-agent to this service and reloads so the change takes
    /// effect on the next navigation.
    pub fn set_user_agent(&self, custom: Option<&str>) {
        let ua = user_agent::for_service(&self.home, custom);
        if let Some(settings) = webkit6::prelude::WebViewExt::settings(&self.webview) {
            settings.set_user_agent(Some(&ua));
        }
        self.webview.reload();
    }
}

/// Returns the last favicon cached for a service, without creating its WebView.
pub fn cached_favicon(session_dir: &Path) -> Option<gdk::Texture> {
    favicon::load_cached(session_dir)
}

/// The user-agent a service resolves to for the given custom value (`None` =
/// the built-in default). Used to show the effective default in the UI.
pub fn resolve_user_agent(url: &str, custom: Option<&str>) -> String {
    user_agent::for_service(url, custom)
}

fn build_settings(user_agent: &str, url: &str) -> webkit6::Settings {
    let settings = webkit6::Settings::new();
    settings.set_user_agent(Some(user_agent));
    settings.set_enable_developer_extras(true);
    settings.set_enable_smooth_scrolling(true);
    settings.set_media_playback_requires_user_gesture(false);
    settings.set_javascript_can_access_clipboard(true);
    // Heavy SPAs such as Google Chat can consume gigabytes and stop painting
    // when forced through WebKitGTK's software path. Keep acceleration as the
    // default, with an escape hatch for machines where GPU compositing fails.
    if std::env::var_os("SYLTR_SW_RENDER").is_some() {
        settings.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Never);
    }
    let media_capture = media_capture_enabled(url, std::env::var_os("SYLTR_TEAMS_CALLS").is_some());
    settings.set_enable_media_stream(media_capture);
    settings.set_enable_webrtc(media_capture);
    enable_runtime_features(&settings);
    settings
}

fn media_capture_enabled(url: &str, enable_teams_calls: bool) -> bool {
    !url.contains("teams.microsoft.com") || enable_teams_calls
}

fn enable_runtime_features(settings: &webkit6::Settings) {
    let Some(list) = webkit6::Settings::all_features() else {
        return;
    };
    for i in 0..list.length() {
        let Some(feature) = list.get(i) else { continue };
        let id = feature
            .identifier()
            .map(|s| s.to_string())
            .unwrap_or_default();
        if id.to_lowercase().contains("idlecallback") {
            settings.set_feature_enabled(&feature, true);
        }
    }
}

/// Applies spell checking to the webview's shared context in the given
/// languages (an empty list turns it off). Backend: enchant/hunspell, so the
/// system dictionaries are used directly.
fn apply_spell(webview: &webkit6::WebView, langs: &[String]) {
    if let Some(context) = webview.context().or_else(webkit6::WebContext::default) {
        context.set_spell_checking_enabled(!langs.is_empty());
        if !langs.is_empty() {
            let refs: Vec<&str> = langs.iter().map(String::as_str).collect();
            context.set_spell_checking_languages(&refs);
        }
    }
}

/// A clicked link asking for a new window (`target=_blank`, i.e. every link in
/// a chat message) opens in the default browser; JS popups (`window.open`,
/// OAuth/SSO) and same-frame navigation stay in-app.
fn wire_link_clicks(webview: &webkit6::WebView) {
    webview.connect_decide_policy(|_wv, decision, decision_type| {
        if decision_type != webkit6::PolicyDecisionType::NewWindowAction {
            return false;
        }
        let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else {
            return false;
        };
        let Some(action) = nav.navigation_action() else {
            return false;
        };
        if action.navigation_type() != webkit6::NavigationType::LinkClicked {
            return false;
        }
        let Some(uri) = action.request().and_then(|r| r.uri()) else {
            return false;
        };
        open_in_default_browser(&uri);
        decision.ignore();
        true
    });
}

fn open_in_default_browser(uri: &str) {
    let context = None::<&gtk::gio::AppLaunchContext>;
    if let Err(e) = gtk::gio::AppInfo::launch_default_for_uri(uri, context) {
        eprintln!("[syltr] could not open {uri} externally: {e}");
    }
}

/// Forwards the site's Web Notifications to the desktop, honoring the service's
/// mute and the global "do not disturb". Notifications arrive from the page via
/// the `syltrNotify` handler (see the shim in [`COMPAT_JS`]) rather than the
/// native Notification API, which WebKit refuses to grant without a user gesture
/// on the isolated per-service sessions.
fn wire_notifications(
    ucm: &webkit6::UserContentManager,
    app: &adw::Application,
    id: &str,
    name: &str,
    dnd: Rc<Cell<bool>>,
    enabled: Rc<Cell<bool>>,
) {
    let app = app.clone();
    let id = id.to_string();
    let name = name.to_string();
    ucm.connect_script_message_received(Some("syltrNotify"), move |_, value| {
        if dnd.get() || !enabled.get() {
            return;
        }
        let payload: serde_json::Value = serde_json::from_str(&value.to_str()).unwrap_or_default();
        let title = payload
            .get("title")
            .and_then(|t| t.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&name);
        let notif = gtk::gio::Notification::new(title);
        if let Some(body) = payload.get("body").and_then(|b| b.as_str()) {
            if !body.is_empty() {
                notif.set_body(Some(body));
            }
        }
        let tag = payload.get("tag").and_then(|t| t.as_str()).unwrap_or("");
        let notif_id = if tag.is_empty() {
            id.clone()
        } else {
            format!("{id}:{tag}")
        };
        app.send_notification(Some(&notif_id), &notif);
    });
}

/// Recovery: if the web process dies (crash/OOM), the page goes blank. Reload
/// ONCE, rate-limited so a page that crashes on load doesn't loop forever.
fn wire_crash_recovery(webview: &webkit6::WebView) {
    let last_reload: Rc<Cell<Option<std::time::Instant>>> = Rc::new(Cell::new(None));
    webview.connect_web_process_terminated(move |wv, reason| {
        eprintln!(
            "syltr[webproc] terminated: {reason:?} — {}",
            wv.uri().unwrap_or_default()
        );
        let now = std::time::Instant::now();
        let looping = last_reload
            .get()
            .is_some_and(|t| now.duration_since(t).as_secs() < 20);
        if !looping {
            last_reload.set(Some(now));
            wv.reload();
        }
    });
}

/// JS error capture (SYLTR_DEBUG=1): forwards console.error/warn, window.onerror
/// and rejected promises to stderr.
fn wire_console_capture(ucm: &webkit6::UserContentManager, name: &str) {
    ucm.register_script_message_handler("consoleCapture", None);
    ucm.add_script(&webkit6::UserScript::new(
        CONSOLE_JS,
        webkit6::UserContentInjectedFrames::AllFrames,
        webkit6::UserScriptInjectionTime::Start,
        &[],
        &[],
    ));
    let tag = name.to_string();
    ucm.connect_script_message_received(Some("consoleCapture"), move |_, value| {
        eprintln!("syltr[{tag}] {}", value.to_str());
    });
}

fn debug_enabled() -> bool {
    std::env::var_os("SYLTR_DEBUG").is_some()
}
