//! CEF client: bundles every per-service handler behind a single Client.

use std::rc::Rc;

use cef::*;

use super::browser_slot::BrowserSlot;
use super::context_menu::ContextMenuHandlerBuilder;
use super::display::DisplayHandlerBuilder;
use super::download::DownloadHandlerBuilder;
use super::lifespan::LifeSpanHandlerBuilder;
use super::permission::PermissionHandlerBuilder;
use super::render::{RenderHandlerBuilder, RenderState};
use super::request::ServiceRequestHandlerBuilder;
use crate::icon::ServiceIcon;

wrap_client! {
    pub(crate) struct ClientBuilder {
        render_handler: RenderHandler,
        display_handler: DisplayHandler,
        life_span_handler: LifeSpanHandler,
        permission_handler: PermissionHandler,
        context_menu_handler: ContextMenuHandler,
        download_handler: DownloadHandler,
        request_handler: RequestHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render_handler.clone())
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display_handler.clone())
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span_handler.clone())
        }
        fn permission_handler(&self) -> Option<PermissionHandler> {
            Some(self.permission_handler.clone())
        }
        fn context_menu_handler(&self) -> Option<ContextMenuHandler> {
            Some(self.context_menu_handler.clone())
        }
        fn download_handler(&self) -> Option<DownloadHandler> {
            Some(self.download_handler.clone())
        }
        fn request_handler(&self) -> Option<RequestHandler> {
            Some(self.request_handler.clone())
        }
    }
}

impl ClientBuilder {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build(
        state: Rc<RenderState>,
        slot: Rc<BrowserSlot>,
        icon: ServiceIcon,
        muted: bool,
        spell_langs: Vec<String>,
        ctx: Option<RequestContext>,
    ) -> Client {
        Self::new(
            RenderHandlerBuilder::build(state.clone()),
            DisplayHandlerBuilder::build(state.clone(), icon),
            LifeSpanHandlerBuilder::build(slot, muted, spell_langs),
            PermissionHandlerBuilder::build(),
            ContextMenuHandlerBuilder::build(state),
            DownloadHandlerBuilder::build(),
            ServiceRequestHandlerBuilder::build(ctx),
        )
    }
}
