//! Per-service request handler: decides which navigations leave for the system
//! browser, and delegates resource handling to the image proxy.

use cef::*;

use super::navigation;
use crate::imgproxy;

wrap_request_handler! {
    pub(crate) struct ServiceRequestHandlerBuilder {
        home: String,
        ctx: Option<RequestContext>,
    }

    impl RequestHandler {
        fn on_before_browse(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            user_gesture: ::std::os::raw::c_int,
            is_redirect: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int {
            // Only pop out navigations the user explicitly started. Redirects
            // (SSO bounce-back) and page/JS navigations always stay in-app.
            if user_gesture == 0 || is_redirect != 0 {
                return 0;
            }
            // Ignore iframes; only the top-level document leaves.
            if frame.as_deref().is_some_and(|f| f.is_main() != 1) {
                return 0;
            }
            let current = frame
                .as_deref()
                .map(|f| CefString::from(&f.url()).to_string());
            let Some(request) = request else { return 0 };
            let url = CefString::from(&request.url()).to_string();
            let dest = navigation::external_target(&url, &self.home, current.as_deref());
            if std::env::var_os("SYLTR_DEBUG").is_some() {
                eprintln!(
                    "[syltr] browse url={url} gesture={user_gesture} redirect={is_redirect} -> {}",
                    dest.as_deref().unwrap_or("in-app"),
                );
            }
            if let Some(dest) = dest {
                navigation::open_external(&dest);
                return 1; // cancel the in-app navigation
            }
            0
        }

        #[allow(clippy::too_many_arguments)]
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _is_navigation: ::std::os::raw::c_int,
            _is_download: ::std::os::raw::c_int,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut ::std::os::raw::c_int>,
        ) -> Option<ResourceRequestHandler> {
            let url = request
                .map(|r| CefString::from(&r.url()).to_string())
                .unwrap_or_default();
            imgproxy::resource_request_handler(&url, self.ctx.clone())
        }
    }
}

impl ServiceRequestHandlerBuilder {
    pub(crate) fn build(home: String, ctx: Option<RequestContext>) -> RequestHandler {
        Self::new(home, ctx)
    }
}
