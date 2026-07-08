//! Handlers CEF para OSR (offscreen rendering) + ponte com o Cairo do GTK.
//!
//! O CEF renderiza a página fora de tela e entrega o buffer BGRA em `on_paint`.
//! Guardamos esse buffer num `thread_local` e o `draw()` (chamado pelo
//! `DrawingArea` do GTK) o desenha via Cairo. Tudo roda na mesma thread (o
//! message loop do CEF é bombeado a partir do loop do GLib), então não há
//! travessia de threads.

use std::cell::{Cell, RefCell};

use cef::{
    rc::Rc, BrowserProcessHandler, ImplBrowserProcessHandler, ImplRenderHandler,
    ImplRequestContextHandler, RenderHandler, RequestContextHandler, WrapBrowserProcessHandler,
    WrapRenderHandler, WrapRequestContextHandler, *,
};
use gtk::cairo;
use gtk::prelude::*;

/// Último frame renderizado pelo CEF (BGRA, largura, altura).
struct Frame {
    data: Vec<u8>,
    w: i32,
    h: i32,
}

thread_local! {
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
    /// Tamanho lógico da view (w, h) e fator de escala — lidos por view_rect/screen_info.
    static VIEW: Cell<(i32, i32, f32)> = const { Cell::new((1000, 720, 1.0)) };
}

/// Atualiza o tamanho da view (chamado no resize do widget GTK).
pub fn set_view_size(w: i32, h: i32) {
    VIEW.with(|v| {
        let (_, _, s) = v.get();
        v.set((w.max(1), h.max(1), s));
    });
}

pub fn set_scale(scale: f32) {
    VIEW.with(|v| {
        let (w, h, _) = v.get();
        v.set((w, h, scale));
    });
}

/// Desenha o último frame do CEF no contexto Cairo do DrawingArea.
pub fn draw(cr: &cairo::Context, _w: i32, _h: i32) {
    FRAME.with_borrow(|frame| {
        let Some(frame) = frame else { return };
        if frame.w <= 0 || frame.h <= 0 {
            return;
        }
        let Ok(mut surface) = cairo::ImageSurface::create(cairo::Format::ARgb32, frame.w, frame.h)
        else {
            return;
        };
        let sstride = surface.stride() as usize;
        let rstride = frame.w as usize * 4;
        if let Ok(mut sdata) = surface.data() {
            for y in 0..frame.h as usize {
                let src = &frame.data[y * rstride..y * rstride + rstride];
                sdata[y * sstride..y * sstride + rstride].copy_from_slice(src);
            }
        }
        surface.mark_dirty();
        let _ = cr.set_source_surface(&surface, 0.0, 0.0);
        let _ = cr.paint();
    });
}

// ---------------------------------------------------------------------------
// App + BrowserProcessHandler
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SyltrApp {}
impl SyltrApp {
    pub fn new() -> Self {
        Self {}
    }
}

wrap_app! {
    pub struct AppBuilder {
        app: SyltrApp,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefStringUtf16>,
            command_line: Option<&mut CommandLine>,
        ) {
            let Some(cmd) = command_line else { return };
            cmd.append_switch(Some(&"no-sandbox".into()));
            cmd.append_switch(Some(&"disable-gpu".into()));
            cmd.append_switch(Some(&"enable-logging=stderr".into()));
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(BrowserProcessHandlerBuilder::build(SyltrBrowserProcessHandler {}))
        }
    }
}

impl AppBuilder {
    pub fn build(app: SyltrApp) -> cef::App {
        Self::new(app)
    }
}

#[derive(Clone)]
pub struct SyltrBrowserProcessHandler {}

wrap_browser_process_handler! {
    pub struct BrowserProcessHandlerBuilder {
        handler: SyltrBrowserProcessHandler,
    }

    impl BrowserProcessHandler {
        fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
            let Some(cmd) = command_line else { return };
            cmd.append_switch(Some(&"no-sandbox".into()));
        }
    }
}

impl BrowserProcessHandlerBuilder {
    pub fn build(handler: SyltrBrowserProcessHandler) -> BrowserProcessHandler {
        Self::new(handler)
    }
}

// ---------------------------------------------------------------------------
// RenderHandler (OSR)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SyltrRenderHandler {}

wrap_render_handler! {
    pub struct RenderHandlerBuilder {
        handler: SyltrRenderHandler,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                let (w, h, _) = VIEW.with(|v| v.get());
                rect.x = 0;
                rect.y = 0;
                rect.width = w;
                rect.height = h;
            }
        }

        fn screen_info(
            &self,
            _browser: Option<&mut Browser>,
            screen_info: Option<&mut ScreenInfo>,
        ) -> ::std::os::raw::c_int {
            if let Some(si) = screen_info {
                si.device_scale_factor = VIEW.with(|v| v.get().2);
                return 1;
            }
            0
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            _type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int,
        ) {
            if buffer.is_null() || width <= 0 || height <= 0 {
                return;
            }
            let size = width as usize * height as usize * 4;
            let slice = unsafe { std::slice::from_raw_parts(buffer, size) };
            FRAME.with_borrow_mut(|frame| match frame {
                Some(f) if f.w == width && f.h == height => {
                    f.data.copy_from_slice(slice);
                }
                _ => {
                    *frame = Some(Frame {
                        data: slice.to_vec(),
                        w: width,
                        h: height,
                    });
                }
            });
        }
    }
}

impl RenderHandlerBuilder {
    pub fn build(handler: SyltrRenderHandler) -> RenderHandler {
        Self::new(handler)
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

wrap_client! {
    pub struct ClientBuilder {
        render_handler: RenderHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render_handler.clone())
        }
    }
}

impl ClientBuilder {
    pub fn build(handler: SyltrRenderHandler) -> Client {
        Self::new(RenderHandlerBuilder::build(handler))
    }
}

// ---------------------------------------------------------------------------
// RequestContextHandler
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SyltrRequestContextHandler {}

wrap_request_context_handler! {
    pub struct RequestContextHandlerBuilder {
        handler: SyltrRequestContextHandler,
    }

    impl RequestContextHandler {}
}

impl RequestContextHandlerBuilder {
    pub fn build(handler: SyltrRequestContextHandler) -> RequestContextHandler {
        Self::new(handler)
    }
}
