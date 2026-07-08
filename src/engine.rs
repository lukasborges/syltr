//! Camada de engine web — agora **CEF (Chromium)** em modo offscreen (OSR).
//!
//! O resto do app usa apenas a API pública de [`ServiceView`]; a integração
//! com o CEF (multiprocesso, message loop, OSR→Cairo, input) fica contida
//! aqui. Cada serviço é um browser CEF windowless renderizando num
//! `GtkDrawingArea`, com sessão/cache isolados por serviço.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use cef::{
    args::Args, rc::Rc as _, App, Browser, BrowserHost, BrowserProcessHandler, BrowserSettings,
    Client, CommandLine, CursorInfo, CursorType, DisplayHandler, ImplBrowser, ImplBrowserHost,
    ImplFrame, LifeSpanHandler, RenderHandler, RequestContextHandler, RequestContextSettings,
    Settings, WindowInfo, *,
};
use gtk::cairo;
use gtk::prelude::*;

use crate::icon::ServiceIcon;
use crate::input;

// ===========================================================================
// Bootstrap global do CEF
// ===========================================================================

/// Inicializa o CEF. Retorna `true` no processo browser (o app deve seguir) e
/// `false` num subprocesso do CEF (o `main` deve sair imediatamente).
/// DEVE ser chamado bem no início do `main`, antes de GTK/libadwaita.
pub fn init_cef() -> bool {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();
    let cmd = args.as_cmd_line().unwrap();
    let is_browser_process = cmd.has_switch(Some(&CefString::from("type"))) != 1;

    let mut app = AppBuilder::build(SyltrApp {});
    let ret = execute_process(Some(args.as_main_args()), Some(&mut app), std::ptr::null_mut());
    if !is_browser_process {
        return false;
    }
    assert_eq!(ret, -1, "processo browser não deveria ser um subprocesso");

    // Raiz dos caches: cada serviço terá seu cache_path como subdiretório
    // disto (exigência do runtime Chrome do CEF para criar perfis).
    let root_cache = gtk::glib::user_data_dir().join(crate::APP_ID).join("sessions");
    let _ = std::fs::create_dir_all(&root_cache);

    let mut settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        root_cache_path: CefString::from(root_cache.to_str().unwrap_or_default()),
        ..Default::default()
    };
    // Recursos do CEF: via CEF_PATH em dev; empacotados junto do binário no
    // deploy (Fase E). Sem a variável, o CEF procura ao lado do executável.
    if let Ok(dir) = std::env::var("CEF_PATH") {
        if !dir.is_empty() {
            settings.resources_dir_path = CefString::from(dir.as_str());
            settings.locales_dir_path = CefString::from(format!("{dir}/locales").as_str());
        }
    }
    assert_eq!(
        initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut()),
        1,
        "falha ao inicializar o CEF"
    );
    true
}

/// Bomba o message loop do CEF a partir do loop do GLib. Chamar uma vez.
pub fn start_pump() {
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(15), || {
        do_message_loop_work();
        gtk::glib::ControlFlow::Continue
    });
}

/// Encerra o CEF (chamar ao sair).
pub fn shutdown_cef() {
    shutdown();
}

// ===========================================================================
// App + BrowserProcessHandler (globais)
// ===========================================================================

#[derive(Clone)]
struct SyltrApp {}

wrap_app! {
    struct AppBuilder {
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
            cmd.append_switch(Some(&"enable-logging=stderr".into()));
            // Conecta o clipboard do Chromium ao do sistema no OSR (Linux).
            cmd.append_switch_with_value(Some(&"ozone-platform".into()), Some(&"x11".into()));
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(BrowserProcessHandlerBuilder::build(SyltrBrowserProcessHandler {}))
        }
    }
}

impl AppBuilder {
    fn build(app: SyltrApp) -> App {
        Self::new(app)
    }
}

#[derive(Clone)]
struct SyltrBrowserProcessHandler {}

wrap_browser_process_handler! {
    struct BrowserProcessHandlerBuilder {
        handler: SyltrBrowserProcessHandler,
    }

    impl BrowserProcessHandler {
        fn on_before_child_process_launch(&self, command_line: Option<&mut CommandLine>) {
            if let Some(cmd) = command_line {
                cmd.append_switch(Some(&"no-sandbox".into()));
            }
        }
    }
}

impl BrowserProcessHandlerBuilder {
    fn build(handler: SyltrBrowserProcessHandler) -> BrowserProcessHandler {
        Self::new(handler)
    }
}

// ===========================================================================
// Estado de render por serviço + desenho Cairo
// ===========================================================================

struct PaintBuffer {
    data: Vec<u8>,
    w: i32,
    h: i32,
}

struct RenderState {
    buffer: RefCell<Option<PaintBuffer>>,
    /// (largura, altura, fator de escala) lógicos da view
    size: Cell<(i32, i32, f32)>,
    area: gtk::DrawingArea,
}

/// Browser/host preenchidos assincronamente em `on_after_created`.
pub struct BrowserSlot {
    browser: RefCell<Option<Browser>>,
    host: RefCell<Option<BrowserHost>>,
}

impl BrowserSlot {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            browser: RefCell::new(None),
            host: RefCell::new(None),
        })
    }
    pub fn host(&self) -> Option<BrowserHost> {
        self.host.borrow().clone()
    }
    fn browser(&self) -> Option<Browser> {
        self.browser.borrow().clone()
    }
    pub fn main_frame(&self) -> Option<Frame> {
        self.browser().and_then(|b| b.main_frame())
    }
}

fn draw(state: &RenderState, cr: &cairo::Context) {
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
    let _ = cr.set_source_surface(&surface, 0.0, 0.0);
    let _ = cr.paint();
}

// ===========================================================================
// RenderHandler + DisplayHandler + Client + RequestContextHandler (por serviço)
// ===========================================================================

wrap_render_handler! {
    struct RenderHandlerBuilder {
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
    fn build(state: Rc<RenderState>) -> RenderHandler {
        Self::new(state)
    }
}

wrap_display_handler! {
    struct DisplayHandlerBuilder {
        state: Rc<RenderState>,
        icon: ServiceIcon,
    }

    impl DisplayHandler {
        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: ::std::os::raw::c_ulong,
            type_: CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> ::std::os::raw::c_int {
            self.state.area.set_cursor_from_name(Some(cursor_name(type_)));
            1
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            if let Some(title) = title {
                self.icon.set_badge(unread_from_title(&title.to_string()));
            }
        }
    }
}

impl DisplayHandlerBuilder {
    fn build(state: Rc<RenderState>, icon: ServiceIcon) -> DisplayHandler {
        Self::new(state, icon)
    }
}

/// Extrai a contagem de não lidas do título (ex.: "(5) WhatsApp",
/// "Inbox (12) - ...", "5 mensagens"). 0 se não achar.
fn unread_from_title(title: &str) -> u32 {
    let bytes = title.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if c == b'(' || c == b'[' || c == b'{' {
            let mut n = 0u32;
            let mut found = false;
            for &d in &bytes[i + 1..] {
                if d.is_ascii_digit() {
                    n = n.saturating_mul(10).saturating_add((d - b'0') as u32);
                    found = true;
                } else {
                    break;
                }
            }
            if found {
                return n;
            }
        }
    }
    // Número no início do título.
    let mut n = 0u32;
    let mut found = false;
    for &d in bytes {
        if d.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add((d - b'0') as u32);
            found = true;
        } else {
            break;
        }
    }
    if found {
        n
    } else {
        0
    }
}

wrap_life_span_handler! {
    struct LifeSpanHandlerBuilder {
        slot: Rc<BrowserSlot>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                let host = browser.host();
                *self.slot.browser.borrow_mut() = Some(browser.clone());
                *self.slot.host.borrow_mut() = host.clone();
                if let Some(host) = host {
                    host.was_resized();
                }
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
            // Cancela a nova janela e carrega a URL na própria view do serviço
            // (login "Entrar com Google" etc. abre in-place).
            if let Some(url) = target_url {
                if let Some(frame) = frame {
                    frame.load_url(Some(url));
                } else if let Some(frame) =
                    browser.and_then(|b| b.main_frame())
                {
                    frame.load_url(Some(url));
                }
            }
            1
        }
    }
}

impl LifeSpanHandlerBuilder {
    fn build(slot: Rc<BrowserSlot>) -> LifeSpanHandler {
        Self::new(slot)
    }
}

wrap_client! {
    struct ClientBuilder {
        render_handler: RenderHandler,
        display_handler: DisplayHandler,
        life_span_handler: LifeSpanHandler,
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
    }
}

impl ClientBuilder {
    fn build(state: Rc<RenderState>, slot: Rc<BrowserSlot>, icon: ServiceIcon) -> Client {
        Self::new(
            RenderHandlerBuilder::build(state.clone()),
            DisplayHandlerBuilder::build(state, icon),
            LifeSpanHandlerBuilder::build(slot),
        )
    }
}

#[derive(Clone)]
struct SyltrRequestContextHandler {}

wrap_request_context_handler! {
    struct RequestContextHandlerBuilder {
        handler: SyltrRequestContextHandler,
    }

    impl RequestContextHandler {}
}

impl RequestContextHandlerBuilder {
    fn build(handler: SyltrRequestContextHandler) -> RequestContextHandler {
        Self::new(handler)
    }
}

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
    } else if t == CursorType::COLUMNRESIZE {
        "col-resize"
    } else if t == CursorType::ROWRESIZE {
        "row-resize"
    } else if t == CursorType::EASTWESTRESIZE {
        "ew-resize"
    } else if t == CursorType::NORTHSOUTHRESIZE {
        "ns-resize"
    } else if t == CursorType::ZOOMIN {
        "zoom-in"
    } else if t == CursorType::NONE {
        "none"
    } else {
        "default"
    }
}

// ===========================================================================
// ServiceView — API pública (igual à versão WebKit)
// ===========================================================================

#[derive(Clone)]
pub struct ServiceView {
    root: gtk::Widget,
    slot: Rc<BrowserSlot>,
    icon: ServiceIcon,
    muted: Rc<Cell<bool>>,
    home: String,
}

impl ServiceView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _id: &str,
        name: &str,
        url: &str,
        session_dir: &Path,
        _app: &adw::Application,
        _dnd: Rc<Cell<bool>>,
        muted: bool,
        _spell_langs: &[String],
        _media_enabled: bool,
    ) -> Self {
        let area = gtk::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let state = Rc::new(RenderState {
            buffer: RefCell::new(None),
            size: Cell::new((800, 600, 1.0)),
            area: area.clone(),
        });

        {
            let state = state.clone();
            area.set_draw_func(move |_, cr, _w, _h| draw(&state, cr));
        }

        // Sessão/cache isolados por serviço. O caminho é o próprio session_dir
        // (subdiretório de root_cache_path), como o runtime Chrome exige.
        let _ = std::fs::create_dir_all(session_dir);
        let rc_settings = RequestContextSettings {
            cache_path: CefString::from(session_dir.to_str().unwrap_or_default()),
            ..Default::default()
        };
        let mut context = request_context_create_context(
            Some(&rc_settings),
            Some(&mut RequestContextHandlerBuilder::build(SyltrRequestContextHandler {})),
        );

        let window_info = WindowInfo {
            windowless_rendering_enabled: 1,
            ..Default::default()
        };
        let browser_settings = BrowserSettings {
            windowless_frame_rate: 60,
            // Fundo branco opaco (senão páginas sem fundo próprio ficam pretas
            // no OSR). cef_color_t é ARGB: 0xFFFFFFFF = branco opaco.
            background_color: 0xFFFF_FFFF,
            ..Default::default()
        };
        let icon = ServiceIcon::new(name);
        let slot = BrowserSlot::new();
        // Criação ASSÍNCRONA: o runtime Chrome cria o Profile por serviço de
        // forma assíncrona, então o browser chega em on_after_created (slot).
        browser_host_create_browser(
            Some(&window_info),
            Some(&mut ClientBuilder::build(state.clone(), slot.clone(), icon.clone())),
            Some(&CefString::from(url)),
            Some(&browser_settings),
            None,
            context.as_mut(),
        );

        // Redimensionamento: atualiza a view e avisa o CEF.
        {
            let state = state.clone();
            let slot = slot.clone();
            area.connect_resize(move |_, w, h| {
                let scale = state.size.get().2;
                state.size.set((w.max(1), h.max(1), scale));
                if let Some(host) = slot.host() {
                    host.was_resized();
                }
            });
        }

        input::attach(&area, slot.clone());

        let root: gtk::Widget = area.upcast();

        Self {
            root,
            slot,
            icon,
            muted: Rc::new(Cell::new(muted)),
            home: url.to_string(),
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }

    pub fn icon(&self) -> &gtk::Widget {
        self.icon.widget()
    }

    pub fn reload(&self) {
        if let Some(browser) = self.slot.browser() {
            browser.reload();
        }
    }

    pub fn go_home(&self) {
        if let Some(frame) = self.slot.browser().and_then(|b| b.main_frame()) {
            frame.load_url(Some(&CefString::from(self.home.as_str())));
        }
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.set(muted);
    }

    /// (Fase D) verificação ortográfica — CEF tem spellcheck próprio.
    pub fn set_spell_languages(&self, _langs: &[String]) {}

    /// (Fase D) mídia/chamadas — no Chromium funcionam nativamente.
    pub fn set_media_enabled(&self, _enabled: bool) {}
}
