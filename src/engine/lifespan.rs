//! Life-span handler: captures the browser/host once created, applies the
//! profile-dependent settings, and keeps popups in-place.

use std::rc::Rc;

use cef::rc::Rc as _;
use cef::*;

use super::browser_slot::BrowserSlot;
use super::prefs::apply_spell_prefs;

wrap_life_span_handler! {
    pub(crate) struct LifeSpanHandlerBuilder {
        slot: Rc<BrowserSlot>,
        muted: bool,
        spell_langs: Vec<String>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(browser) = browser else { return };
            let host = browser.host();
            self.slot.fill(browser.clone(), host.clone());

            let Some(host) = host else { return };
            host.was_resized();
            // The profile is only ready now; doing this in new() is too early
            // (can_set_preference returns false there).
            if let Some(ctx) = host.request_context() {
                let value = if self.muted {
                    ContentSettingValues::BLOCK
                } else {
                    ContentSettingValues::ALLOW
                };
                ctx.set_content_setting(None, None, ContentSettingTypes::NOTIFICATIONS, value);
                apply_spell_prefs(&ctx, &self.spell_langs);
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn on_before_popup(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _popup_id: ::std::os::raw::c_int,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition,
            _user_gesture: ::std::os::raw::c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
        ) -> ::std::os::raw::c_int {
            // Cancel the new window and load the URL in the service's own view,
            // so "Sign in with Google" and similar flows open in-place.
            if let Some(url) = target_url {
                if let Some(frame) = frame {
                    frame.load_url(Some(url));
                } else if let Some(frame) = browser.and_then(|b| b.main_frame()) {
                    frame.load_url(Some(url));
                }
            }
            1
        }
    }
}

impl LifeSpanHandlerBuilder {
    pub(crate) fn build(slot: Rc<BrowserSlot>, muted: bool, spell_langs: Vec<String>) -> LifeSpanHandler {
        Self::new(slot, muted, spell_langs)
    }
}
