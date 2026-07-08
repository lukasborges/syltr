//! Fase A do spike: CEF (Chromium) renderizando offscreen dentro de uma
//! janela GTK4, via Cairo. Prova a integração antes de reescrever o engine.rs.

mod osr;

use std::process::ExitCode;
use std::time::Duration;

use cef::{args::Args, *};
use gtk::prelude::*;
use gtk::glib;

use osr::{AppBuilder, ClientBuilder, RequestContextHandlerBuilder, SyltrApp,
          SyltrRenderHandler, SyltrRequestContextHandler};

const START_URL: &str = "https://web.telegram.org/";

fn main() -> ExitCode {
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    // Modelo multiprocesso do CEF: o mesmo binário é re-executado como
    // subprocesso (render/gpu/etc). execute_process trata isso.
    let args = Args::new();
    let cmd = args.as_cmd_line().unwrap();
    let is_browser_process = cmd.has_switch(Some(&CefString::from("type"))) != 1;

    let mut app = AppBuilder::build(SyltrApp::new());
    let ret = execute_process(Some(args.as_main_args()), Some(&mut app), std::ptr::null_mut());
    if !is_browser_process {
        // Subprocesso: não inicializa CEF nem GTK.
        return ExitCode::from(0);
    }
    assert_eq!(ret, -1, "processo browser não deveria ser um subprocesso");

    // Onde estão os recursos do CEF (.pak, icudtl.dat, locales/).
    let cef_dir = std::env::var("CEF_PATH").expect("defina CEF_PATH para a distribuição do CEF");
    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        resources_dir_path: CefString::from(cef_dir.as_str()),
        locales_dir_path: CefString::from(format!("{cef_dir}/locales").as_str()),
        ..Default::default()
    };
    assert_eq!(
        initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut()),
        1,
        "falha ao inicializar o CEF"
    );

    gtk::init().expect("gtk init");

    let area = gtk::DrawingArea::builder().hexpand(true).vexpand(true).build();
    area.set_draw_func(|_, cr, w, h| osr::draw(cr, w, h));

    let window = gtk::Window::builder()
        .title("Syltr — CEF spike (offscreen)")
        .default_width(1000)
        .default_height(720)
        .child(&area)
        .build();

    // Cria o browser CEF em modo windowless (OSR).
    let window_info = WindowInfo {
        windowless_rendering_enabled: 1,
        ..Default::default()
    };
    let browser_settings = BrowserSettings {
        windowless_frame_rate: 60,
        ..Default::default()
    };
    let mut context = request_context_create_context(
        Some(&RequestContextSettings::default()),
        Some(&mut RequestContextHandlerBuilder::build(SyltrRequestContextHandler {})),
    );
    let browser = browser_host_create_browser_sync(
        Some(&window_info),
        Some(&mut ClientBuilder::build(SyltrRenderHandler {})),
        Some(&CefString::from(START_URL)),
        Some(&browser_settings),
        None,
        context.as_mut(),
    )
    .expect("falha ao criar o browser CEF");

    // Redimensionamento: informa o CEF do novo tamanho da view.
    {
        let host = browser.host();
        area.connect_resize(move |_, w, h| {
            osr::set_view_size(w, h);
            if let Some(host) = &host {
                host.was_resized();
            }
        });
    }

    let main_loop = glib::MainLoop::new(None, false);
    {
        let ml = main_loop.clone();
        window.connect_close_request(move |_| {
            ml.quit();
            glib::Propagation::Proceed
        });
    }
    window.present();

    // Bomba o message loop do CEF a partir do loop do GLib e redesenha.
    {
        let area = area.clone();
        glib::timeout_add_local(Duration::from_millis(15), move || {
            do_message_loop_work();
            area.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    main_loop.run();

    let _ = browser; // mantém o browser vivo até aqui
    cef::shutdown();
    ExitCode::from(0)
}
