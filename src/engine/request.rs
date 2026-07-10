//! Per-service request handler: delegates resource handling to the image proxy.
//! Every navigation stays in-app — nothing is ever opened in an external window.

use cef::*;

use crate::imgproxy;

wrap_request_handler! {
    pub(crate) struct ServiceRequestHandlerBuilder {
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
            imgproxy::resource_request_handler(&url, self.ctx.clone())
        }
    }
}

impl ServiceRequestHandlerBuilder {
    pub(crate) fn build(ctx: Option<RequestContext>) -> RequestHandler {
        Self::new(ctx)
    }
}
