//! Global CEF bootstrap: process initialization, the App and the
//! BrowserProcessHandler, and driving the message loop from GLib.

use cef::{args::Args, *};

/// Initializes CEF. Returns `true` in the browser process (the app should carry
/// on) and `false` in a CEF subprocess (`main` must exit immediately). MUST be
/// called at the very start of `main`, before GTK/libadwaita.
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
    assert_eq!(ret, -1, "the browser process should not be a subprocess");

    // Cache root: each service's cache_path is a subdirectory of this (the CEF
    // Chrome runtime requires it to create per-service profiles).
    let root_cache = gtk::glib::user_data_dir().join(crate::APP_ID).join("sessions");
    let _ = std::fs::create_dir_all(&root_cache);

    let mut settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        root_cache_path: CefString::from(root_cache.to_str().unwrap_or_default()),
        ..Default::default()
    };
    // CEF resources: via CEF_PATH in dev; bundled next to the binary in deploy.
    // Without the variable, CEF looks beside the executable.
    if let Ok(dir) = std::env::var("CEF_PATH") {
        if !dir.is_empty() {
            settings.resources_dir_path = CefString::from(dir.as_str());
            settings.locales_dir_path = CefString::from(format!("{dir}/locales").as_str());
        }
    }
    assert_eq!(
        initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut()),
        1,
        "failed to initialize CEF"
    );
    true
}

/// Pumps the CEF message loop from the GLib loop. Call once.
pub fn start_pump() {
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(15), || {
        do_message_loop_work();
        gtk::glib::ControlFlow::Continue
    });
}

pub fn shutdown_cef() {
    shutdown();
}

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
            // Wire the Chromium clipboard to the system one in OSR (Linux).
            cmd.append_switch_with_value(Some(&"ozone-platform".into()), Some(&"x11".into()));
            // Document-Isolation-Policy (a new Chromium feature): in CEF 149 it
            // breaks cross-origin images served via Service Worker (custom emoji
            // and Google Chat attachments fail with ERR_INVALID_ARGUMENT).
            cmd.append_switch_with_value(
                Some(&"disable-features".into()),
                Some(&"DocumentIsolationPolicy".into()),
            );
            // Debug hook: arbitrary Chromium switches via env, e.g.
            // SYLTR_CEF_ARGS="disable-quic log-net-log=/tmp/n.json foo=bar".
            if let Some(s) = std::env::var("SYLTR_CEF_ARGS").ok().filter(|s| !s.is_empty()) {
                for tok in s.split_whitespace() {
                    match tok.split_once('=') {
                        Some((k, v)) => cmd.append_switch_with_value(Some(&k.into()), Some(&v.into())),
                        None => cmd.append_switch(Some(&tok.into())),
                    }
                }
            }
            // Remote DevTools only in debug mode (SYLTR_DEBUG=1): localhost:9222.
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
