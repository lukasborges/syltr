//! Handlers CEF para OSR (offscreen rendering) + ponte com o Cairo do GTK.
//!
//! O CEF renderiza a página fora de tela e entrega o buffer BGRA em `on_paint`.
//! Guardamos esse buffer num `thread_local` e o `draw()` (chamado pelo
//! `DrawingArea` do GTK) o desenha via Cairo. Tudo roda na mesma thread (o
//! message loop do CEF é bombeado a partir do loop do GLib), então não há
//! travessia de threads.

use std::cell::{Cell, RefCell};

use cef::{
    rc::Rc, BrowserProcessHandler, DisplayHandler, ImplBrowserProcessHandler, ImplDisplayHandler,
    ImplRenderHandler, ImplRequestContextHandler, RenderHandler, RequestContextHandler,
    WrapBrowserProcessHandler, WrapDisplayHandler, WrapRenderHandler, WrapRequestContextHandler, *,
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
    /// DrawingArea alvo (para trocar o cursor em on_cursor_change).
    static AREA: RefCell<Option<gtk::DrawingArea>> = const { RefCell::new(None) };
}

/// Registra o DrawingArea que recebe as trocas de cursor.
pub fn set_area(area: &gtk::DrawingArea) {
    AREA.with_borrow_mut(|a| *a = Some(area.clone()));
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
        display_handler: DisplayHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render_handler.clone())
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display_handler.clone())
        }
    }
}

impl ClientBuilder {
    pub fn build(handler: SyltrRenderHandler) -> Client {
        Self::new(
            RenderHandlerBuilder::build(handler),
            DisplayHandlerBuilder::build(SyltrDisplayHandler {}),
        )
    }
}

// ---------------------------------------------------------------------------
// DisplayHandler (troca de cursor)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SyltrDisplayHandler {}

wrap_display_handler! {
    pub struct DisplayHandlerBuilder {
        handler: SyltrDisplayHandler,
    }

    impl DisplayHandler {
        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: ::std::os::raw::c_ulong,
            type_: CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> ::std::os::raw::c_int {
            let name = cursor_name(type_);
            AREA.with_borrow(|a| {
                if let Some(area) = a {
                    area.set_cursor_from_name(Some(name));
                }
            });
            1
        }
    }
}

impl DisplayHandlerBuilder {
    pub fn build(handler: SyltrDisplayHandler) -> DisplayHandler {
        Self::new(handler)
    }
}

/// Mapeia o tipo de cursor do CEF para um nome de cursor CSS/GDK.
fn cursor_name(t: CursorType) -> &'static str {
    if t == CursorType::HAND {
        "pointer"
    } else if t == CursorType::IBEAM {
        "text"
    } else if t == CursorType::CROSS {
        "crosshair"
    } else if t == CursorType::WAIT {
        "wait"
    } else if t == CursorType::HELP {
        "help"
    } else if t == CursorType::MOVE {
        "move"
    } else if t == CursorType::PROGRESS {
        "progress"
    } else if t == CursorType::NOTALLOWED {
        "not-allowed"
    } else if t == CursorType::NODROP {
        "no-drop"
    } else if t == CursorType::COPY {
        "copy"
    } else if t == CursorType::CONTEXTMENU {
        "context-menu"
    } else if t == CursorType::CELL {
        "cell"
    } else if t == CursorType::COLUMNRESIZE {
        "col-resize"
    } else if t == CursorType::ROWRESIZE {
        "row-resize"
    } else if t == CursorType::EASTWESTRESIZE {
        "ew-resize"
    } else if t == CursorType::NORTHSOUTHRESIZE {
        "ns-resize"
    } else if t == CursorType::NORTHEASTSOUTHWESTRESIZE {
        "nesw-resize"
    } else if t == CursorType::NORTHWESTSOUTHEASTRESIZE {
        "nwse-resize"
    } else if t == CursorType::EASTRESIZE {
        "e-resize"
    } else if t == CursorType::WESTRESIZE {
        "w-resize"
    } else if t == CursorType::NORTHRESIZE {
        "n-resize"
    } else if t == CursorType::SOUTHRESIZE {
        "s-resize"
    } else if t == CursorType::ZOOMIN {
        "zoom-in"
    } else if t == CursorType::NONE {
        "none"
    } else {
        "default"
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
