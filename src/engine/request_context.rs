//! Empty request-context handler, required to create a per-service context.

use cef::*;

#[derive(Clone)]
struct SyltrRequestContextHandler {}

wrap_request_context_handler! {
    pub(crate) struct RequestContextHandlerBuilder {
        handler: SyltrRequestContextHandler,
    }

    impl RequestContextHandler {}
}

impl RequestContextHandlerBuilder {
    pub(crate) fn build() -> RequestContextHandler {
        Self::new(SyltrRequestContextHandler {})
    }
}
