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
    Client, CommandLine, CursorInfo, CursorType, DisplayHandler, DownloadImageCallback,
    ContentSettingTypes, ContentSettingValues, ContextMenuHandler, ContextMenuParams, EventFlags,
    ImplBinaryValue, ImplBrowser, ImplBrowserHost, ImplContextMenuParams, ImplFrame, ImplImage,
    ImplListValue, ImplMenuModel, ImplPermissionPromptCallback, ImplPreferenceManager,
    ImplRequestContext, ImplRunContextMenuCallback, ImplValue, LifeSpanHandler, MenuItemType,
    MenuModel, PermissionHandler, PermissionRequestResult, RenderHandler, RequestContext,
    RequestContextHandler, RequestContextSettings, RunContextMenuCallback, Settings, WindowInfo, *,
};
use gtk::prelude::*;
use gtk::{cairo, gdk, glib};

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
            // DevTools remoto só em modo debug (SYLTR_DEBUG=1): http://localhost:9222
            if std::env::var_os("SYLTR_DEBUG").is_some() {
                cmd.append_switch_with_value(
                    Some(&"remote-debugging-port".into()),
                    Some(&"9222".into()),
                );
            }
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
    // HiDPI: o buffer do CEF vem em pixels de dispositivo (logical * escala);
    // o device_scale faz o Cairo mapear de volta para o tamanho lógico.
    let scale = state.size.get().2 as f64;
    if scale > 0.0 {
        surface.set_device_scale(scale, scale);
    }
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

        fn on_favicon_urlchange(
            &self,
            browser: Option<&mut Browser>,
            icon_urls: Option<&mut CefStringList>,
        ) {
            let (Some(browser), Some(urls)) = (browser, icon_urls) else { return };
            let Some(host) = browser.host() else { return };
            let list = std::mem::take(urls);
            let Some(url) = list.into_iter().next() else { return };
            let mut callback = FaviconCallbackBuilder::build(self.icon.clone());
            host.download_image(
                Some(&CefString::from(url.as_str())),
                1,  // is_favicon
                64, // max_image_size
                0,  // bypass_cache
                Some(&mut callback),
            );
        }
    }
}

wrap_download_image_callback! {
    struct FaviconCallbackBuilder {
        icon: ServiceIcon,
    }

    impl DownloadImageCallback {
        fn on_download_image_finished(
            &self,
            _image_url: Option<&CefString>,
            _http_status_code: ::std::os::raw::c_int,
            image: Option<&mut cef::Image>,
        ) {
            let Some(image) = image else { return };
            let mut pw = 0;
            let mut ph = 0;
            let Some(png) = image.as_png(1.0, 1, Some(&mut pw), Some(&mut ph)) else {
                return;
            };
            let size = png.size();
            if size == 0 {
                return;
            }
            let bytes = unsafe { std::slice::from_raw_parts(png.raw_data() as *const u8, size) };
            if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from(bytes)) {
                self.icon.set_favicon(Some(&texture));
            }
        }
    }
}

impl FaviconCallbackBuilder {
    fn build(icon: ServiceIcon) -> DownloadImageCallback {
        Self::new(icon)
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
        muted: bool,
        spell_langs: Vec<String>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            if let Some(browser) = browser {
                let host = browser.host();
                *self.slot.browser.borrow_mut() = Some(browser.clone());
                *self.slot.host.borrow_mut() = host.clone();
                if let Some(host) = host {
                    host.was_resized();
                    // Agora o profile está pronto: aplica notificações e corretor
                    // (no new() é cedo demais — can_set_preference retorna false).
                    if let Some(ctx) = host.request_context() {
                        let value = if self.muted {
                            ContentSettingValues::BLOCK
                        } else {
                            ContentSettingValues::ALLOW
                        };
                        ctx.set_content_setting(None, None, ContentSettingTypes::NOTIFICATIONS, value);
                        apply_spell_prefs(&ctx, &self.spell_langs);
                    }
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
    fn build(slot: Rc<BrowserSlot>, muted: bool, spell_langs: Vec<String>) -> LifeSpanHandler {
        Self::new(slot, muted, spell_langs)
    }
}

wrap_client! {
    struct ClientBuilder {
        render_handler: RenderHandler,
        display_handler: DisplayHandler,
        life_span_handler: LifeSpanHandler,
        permission_handler: PermissionHandler,
        context_menu_handler: ContextMenuHandler,
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
    }
}

impl ClientBuilder {
    fn build(
        state: Rc<RenderState>,
        slot: Rc<BrowserSlot>,
        icon: ServiceIcon,
        muted: bool,
        spell_langs: Vec<String>,
    ) -> Client {
        Self::new(
            RenderHandlerBuilder::build(state.clone()),
            DisplayHandlerBuilder::build(state.clone(), icon),
            LifeSpanHandlerBuilder::build(slot, muted, spell_langs),
            PermissionHandlerBuilder::build(SyltrPermissionHandler {}),
            ContextMenuHandlerBuilder::build(state),
        )
    }
}

// ---------------------------------------------------------------------------
// ContextMenuHandler — no OSR o menu nativo não aparece; desenhamos um popover
// GTK a partir do modelo que o CEF fornece (com sugestões de correção,
// copiar/colar, idiomas do corretor, etc.).
// ---------------------------------------------------------------------------

wrap_context_menu_handler! {
    struct ContextMenuHandlerBuilder {
        state: Rc<RenderState>,
    }

    impl ContextMenuHandler {
        fn run_context_menu(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            params: Option<&mut ContextMenuParams>,
            model: Option<&mut MenuModel>,
            callback: Option<&mut RunContextMenuCallback>,
        ) -> ::std::os::raw::c_int {
            let (Some(params), Some(model), Some(callback)) = (params, model, callback) else {
                return 0;
            };
            if model.count() == 0 {
                return 0;
            }
            let (x, y) = (params.xcoord(), params.ycoord());

            let popover = gtk::Popover::new();
            popover.set_parent(&self.state.area);
            popover.set_has_arrow(false);
            popover.add_css_class("menu");
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, 1)));

            let done = Rc::new(Cell::new(false));
            let bx = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .width_request(220)
                .build();
            let scroll = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .max_content_height(500)
                .propagate_natural_height(true)
                .child(&bx)
                .build();
            fill_menu(&bx, model, callback, &done, &popover);
            popover.set_child(Some(&scroll));

            {
                let cb = callback.clone();
                let done = done.clone();
                popover.connect_closed(move |p| {
                    if !done.get() {
                        cb.cancel();
                    }
                    p.unparent();
                });
            }
            popover.popup();
            1
        }
    }
}

impl ContextMenuHandlerBuilder {
    fn build(state: Rc<RenderState>) -> ContextMenuHandler {
        Self::new(state)
    }
}

/// Preenche o popover com botões a partir do CefMenuModel (recursivo p/
/// submenus, achatados com um cabeçalho). Cada botão chama callback.cont(id).
fn fill_menu(
    bx: &gtk::Box,
    model: &MenuModel,
    cb: &RunContextMenuCallback,
    done: &Rc<Cell<bool>>,
    pop: &gtk::Popover,
) {
    for i in 0..model.count() {
        let t = model.type_at(i);
        if t == MenuItemType::SEPARATOR {
            bx.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        } else if t == MenuItemType::SUBMENU {
            if let Some(sub) = model.sub_menu_at(i) {
                let header = gtk::Label::builder()
                    .label(menu_label(model, i))
                    .xalign(0.0)
                    .margin_start(8)
                    .margin_top(4)
                    .css_classes(["dim-label", "caption-heading"])
                    .build();
                bx.append(&header);
                fill_menu(bx, &sub, cb, done, pop);
            }
        } else {
            let mut label = menu_label(model, i);
            if model.is_checked_at(i) != 0 {
                label = format!("\u{2713} {label}");
            }
            let button = gtk::Button::with_label(&label);
            button.add_css_class("flat");
            button.set_sensitive(model.is_enabled_at(i) != 0);
            if let Some(lbl) = button.child().and_downcast::<gtk::Label>() {
                lbl.set_xalign(0.0);
            }
            let id = model.command_id_at(i);
            let cb = cb.clone();
            let done = done.clone();
            let pop = pop.clone();
            button.connect_clicked(move |_| {
                done.set(true);
                cb.cont(id, EventFlags::default());
                pop.popdown();
            });
            bx.append(&button);
        }
    }
}

fn menu_label(model: &MenuModel, i: usize) -> String {
    let raw = CefString::from(&model.label_at(i)).to_string();
    // Remove o marcador de mnemônico estilo Windows ('&'); '&&' vira '&'.
    raw.replace("&&", "\u{1}")
        .replace('&', "")
        .replace('\u{1}', "&")
}

/// Aplica os idiomas do corretor ao contexto (preferências do Chromium).
fn apply_spell_prefs(ctx: &RequestContext, langs: &[String]) {
    let enabled = !langs.is_empty();
    if let Some(mut v) = value_create() {
        v.set_bool(enabled as _);
        ctx.set_preference(
            Some(&CefString::from("browser.enable_spellchecking")),
            Some(&mut v),
            None,
        );
    }
    if let (Some(mut list), Some(mut val)) = (list_value_create(), value_create()) {
        list.set_size(langs.len());
        for (i, lang) in langs.iter().enumerate() {
            // Chromium usa hífen e região: pt_BR -> pt-BR.
            let code = lang.replace('_', "-");
            list.set_string(i, Some(&CefString::from(code.as_str())));
        }
        val.set_list(Some(&mut list));
        ctx.set_preference(
            Some(&CefString::from("spellcheck.dictionaries")),
            Some(&mut val),
            None,
        );
    }
}

#[derive(Clone)]
struct SyltrPermissionHandler {}

wrap_permission_handler! {
    struct PermissionHandlerBuilder {
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
            // Cliente dedicado: concede as permissões (ex.: notificações).
            if let Some(cb) = callback {
                cb.cont(PermissionRequestResult::ACCEPT);
            }
            1
        }
    }
}

impl PermissionHandlerBuilder {
    fn build(handler: SyltrPermissionHandler) -> PermissionHandler {
        Self::new(handler)
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
    context: Option<RequestContext>,
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
        spell_langs: &[String],
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
            Some(&mut ClientBuilder::build(
                state.clone(),
                slot.clone(),
                icon.clone(),
                muted,
                spell_langs.to_vec(),
            )),
            Some(&CefString::from(url)),
            Some(&browser_settings),
            None,
            context.as_mut(),
        );

        // Redimensionamento: atualiza a view e avisa o CEF.
        {
            let state = state.clone();
            let slot = slot.clone();
            area.connect_resize(move |area, w, h| {
                let scale = area.scale_factor().max(1) as f32;
                state.size.set((w.max(1), h.max(1), scale));
                if let Some(host) = slot.host() {
                    host.was_resized();
                }
            });
        }

        input::attach(&area, slot.clone());

        // O estado inicial (notificações + corretor) é aplicado em
        // on_after_created, quando o profile do contexto já está pronto.

        let root: gtk::Widget = area.upcast();

        Self {
            root,
            slot,
            icon,
            context,
            home: url.to_string(),
        }
    }

    /// Liga/desliga as notificações do serviço (mute/não-perturbe), bloqueando
    /// no nível do content-setting do contexto — o Chromium para de exibi-las.
    pub fn set_notifications_enabled(&self, enabled: bool) {
        if let Some(ctx) = &self.context {
            let value = if enabled {
                ContentSettingValues::ALLOW
            } else {
                ContentSettingValues::BLOCK
            };
            ctx.set_content_setting(None, None, ContentSettingTypes::NOTIFICATIONS, value);
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

    /// Define os idiomas do corretor ortográfico (Chromium tem o próprio,
    /// separado do hunspell do sistema). `pt_BR` -> `pt-BR`; o Chromium baixa
    /// o dicionário `.bdic` correspondente.
    pub fn set_spell_languages(&self, langs: &[String]) {
        if let Some(ctx) = &self.context {
            apply_spell_prefs(ctx, langs);
        }
    }

    /// (Fase D) mídia/chamadas — no Chromium funcionam nativamente.
    pub fn set_media_enabled(&self, _enabled: bool) {}
}
