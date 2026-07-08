//! Workaround for a CEF 149 bug: following a cross-origin redirect of a
//! `no-cors` subresource (an image) fails with `ERR_INVALID_ARGUMENT`. Google
//! Chat serves attachments and custom emoji exactly this way:
//!
//!   <img src="chat.google.com/api/get_attachment_url?..">  (same-origin)
//!        -> 302 -> lh7-eu.googleusercontent.com/chat_attachment/..  (cross-origin)
//!
//! The final image loads fine when requested directly; only *following the
//! redirect* in the renderer breaks. So we intercept these URLs and re-fetch
//! them via the network service (see [`fetch`]), which follows the 302 without
//! the bug, returning the bytes as a same-origin 200 the renderer accepts.

mod fetch;

use cef::*;

/// URLs whose `<img>` loading triggers the Chat redirect bug.
fn should_intercept(url: &str) -> bool {
    url.contains("/api/get_attachment_url") || url.contains("/api/get_custom_emoji_image")
}

wrap_resource_request_handler! {
    struct ImgResourceRequestHandlerBuilder {
        ctx: Option<RequestContext>,
    }

    impl ResourceRequestHandler {
        // The trait default is RV_CANCEL, which would abort before reaching
        // resource_handler. We must let it continue.
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            ReturnValue::CONTINUE
        }

        fn resource_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let url = request
                .map(|r| CefString::from(&r.url()).to_string())
                .unwrap_or_default();
            should_intercept(&url).then(|| fetch::handler(self.ctx.clone()))
        }
    }
}

impl ImgResourceRequestHandlerBuilder {
    fn build(ctx: Option<RequestContext>) -> ResourceRequestHandler {
        Self::new(ctx)
    }
}

wrap_request_handler! {
    pub struct ImgRequestHandlerBuilder {
        ctx: Option<RequestContext>,
    }

    impl RequestHandler {
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
            should_intercept(&url).then(|| ImgResourceRequestHandlerBuilder::build(self.ctx.clone()))
        }
    }
}

impl ImgRequestHandlerBuilder {
    pub fn build(ctx: Option<RequestContext>) -> RequestHandler {
        Self::new(ctx)
    }
}
