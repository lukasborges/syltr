//! Permission handler. This client is dedicated to a single service, so it
//! simply grants every prompt (e.g. notifications).

use cef::*;

#[derive(Clone)]
struct SyltrPermissionHandler {}

wrap_permission_handler! {
    pub(crate) struct PermissionHandlerBuilder {
        handler: SyltrPermissionHandler,
    }

    impl PermissionHandler {
        fn on_show_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            _prompt_id: u64,
            _requesting_origin: Option<&CefString>,
            _requested_permissions: u32,
            callback: Option<&mut PermissionPromptCallback>,
        ) -> ::std::os::raw::c_int {
            if let Some(cb) = callback {
                cb.cont(PermissionRequestResult::ACCEPT);
            }
            1
        }
    }
}

impl PermissionHandlerBuilder {
    pub(crate) fn build() -> PermissionHandler {
        Self::new(SyltrPermissionHandler {})
    }
}
