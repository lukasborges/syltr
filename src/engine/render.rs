//! Per-service render state and the CEF render handler that paints the offscreen
//! (OSR) buffer into a GtkDrawingArea via Cairo.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use cef::*;
use gtk::cairo;
use gtk::prelude::*;

/// Logical view size defaulted before the first resize.
const DEFAULT_SIZE: (i32, i32, f32) = (800, 600, 1.0);

struct PaintBuffer {
    data: Vec<u8>,
    w: i32,
    h: i32,
}

pub(crate) struct RenderState {
    buffer: RefCell<Option<PaintBuffer>>,
    /// Logical (width, height, scale factor) of the view.
    size: Cell<(i32, i32, f32)>,
    area: gtk::DrawingArea,
}

impl RenderState {
    pub(crate) fn new(area: gtk::DrawingArea) -> Rc<Self> {
        Rc::new(Self {
            buffer: RefCell::new(None),
            size: Cell::new(DEFAULT_SIZE),
            area,
        })
    }

    pub(crate) fn area(&self) -> &gtk::DrawingArea {
        &self.area
    }

    pub(crate) fn set_size(&self, w: i32, h: i32, scale: f32) {
        self.size.set((w, h, scale));
    }
}

pub(crate) fn draw(state: &RenderState, cr: &cairo::Context) {
    let buf = state.buffer.borrow();
    let Some(buf) = buf.as_ref() else { return };
    if buf.w <= 0 || buf.h <= 0 {
        return;
    }
    let Ok(mut surface) = cairo::ImageSurface::create(cairo::Format::ARgb32, buf.w, buf.h) else {
        return;
    };
    let sstride = surface.stride() as usize;
    let rstride = buf.w as usize * 4;
    if let Ok(mut sdata) = surface.data() {
        for y in 0..buf.h as usize {
            sdata[y * sstride..y * sstride + rstride]
                .copy_from_slice(&buf.data[y * rstride..y * rstride + rstride]);
        }
    }
    surface.mark_dirty();
    // CEF delivers the buffer in device pixels (logical * scale); the device
    // scale makes Cairo map it back to the logical size for HiDPI.
    let scale = state.size.get().2 as f64;
    if scale > 0.0 {
        surface.set_device_scale(scale, scale);
    }
    let _ = cr.set_source_surface(&surface, 0.0, 0.0);
    let _ = cr.paint();
}

wrap_render_handler! {
    pub(crate) struct RenderHandlerBuilder {
        state: Rc<RenderState>,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                let (w, h, _) = self.state.size.get();
                rect.x = 0;
                rect.y = 0;
                rect.width = w.max(1);
                rect.height = h.max(1);
            }
        }

        fn screen_info(
            &self,
            _browser: Option<&mut Browser>,
            screen_info: Option<&mut ScreenInfo>,
        ) -> ::std::os::raw::c_int {
            if let Some(si) = screen_info {
                si.device_scale_factor = self.state.size.get().2;
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
            {
                let mut buf = self.state.buffer.borrow_mut();
                match buf.as_mut() {
                    Some(b) if b.w == width && b.h == height => b.data.copy_from_slice(slice),
                    _ => {
                        *buf = Some(PaintBuffer {
                            data: slice.to_vec(),
                            w: width,
                            h: height,
                        })
                    }
                }
            }
            self.state.area.queue_draw();
        }
    }
}

impl RenderHandlerBuilder {
    pub(crate) fn build(state: Rc<RenderState>) -> RenderHandler {
        Self::new(state)
    }
}
