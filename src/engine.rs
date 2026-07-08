//! Camada de abstração da engine web.
//!
//! Hoje o backend é o **WebKitGTK 6** (engine WebKit), nativo e estável no
//! GNOME/Wayland. O objetivo é que o resto do app (janela, sidebar, ações)
//! use APENAS a API pública de [`ServiceView`] e nunca toque no `webkit6`
//! diretamente — assim a migração futura para **CEF (engine Chromium)** fica
//! contida neste único arquivo: basta reimplementar `ServiceView` com o crate
//! `cef` renderizando offscreen dentro de um `gtk::Widget`.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use base64::Engine;
use gtk::gdk;
use gtk::prelude::*;
use webkit6::prelude::*;

use crate::icon::ServiceIcon;

/// JS injetado ao fim do carregamento: acha o melhor ícone declarado na página,
/// rasteriza num canvas 64x64 (o WebKit renderiza qualquer formato, inclusive
/// SVG) e posta o data URL PNG de volta via message handler `faviconReady`.
const FAVICON_JS: &str = r#"
(function () {
  try {
    const links = [...document.querySelectorAll(
      'link[rel~="icon"],link[rel="shortcut icon"],link[rel="apple-touch-icon"]')];
    const apple = links.find(l => (l.rel || '').includes('apple'));
    const svg = links.find(l => ((l.type || '').includes('svg')) || (l.href || '').endsWith('.svg'));
    const href = (apple && apple.href) || (svg && svg.href)
      || (links[0] && links[0].href) || (location.origin + '/favicon.ico');
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      try {
        const c = document.createElement('canvas');
        c.width = 64; c.height = 64;
        const ctx = c.getContext('2d');
        ctx.clearRect(0, 0, 64, 64);
        ctx.drawImage(img, 0, 0, 64, 64);
        window.webkit.messageHandlers.faviconReady.postMessage(c.toDataURL('image/png'));
      } catch (e) {}
    };
    img.src = href;
  } catch (e) {}
})();
"#;

/// JS injetado uma vez: observa o título (e um intervalo de reserva) e posta a
/// contagem de não lidas. Heurística genérica: número entre parênteses/colchetes
/// ou no início do título (ex.: "(5) WhatsApp", "Inbox (12) - ...").
const UNREAD_JS: &str = r#"
(function () {
  if (window.__syltrUnread) return;
  window.__syltrUnread = true;
  const post = () => {
    try {
      let n = 0;
      const t = document.title || '';
      const m = t.match(/[\(\[\{]\s*(\d+)\+?\s*[\)\]\}]/) || t.match(/^\s*(\d+)\s+/);
      if (m) n = parseInt(m[1], 10) || 0;
      window.webkit.messageHandlers.unreadCount.postMessage(n);
    } catch (e) {}
  };
  post();
  try {
    const el = document.querySelector('title');
    if (el) new MutationObserver(post).observe(el,
      { childList: true, characterData: true, subtree: true });
  } catch (e) {}
  setInterval(post, 4000);
})();
"#;

/// Script injetado no início de cada página: faz o site enxergar a permissão
/// de notificação como já concedida. Sem isso, serviços como o WhatsApp Web
/// mostram "notificações desativadas" a cada abertura, porque o WebKit não
/// persiste a permissão entre sessões. As notificações reais seguem
/// funcionando pelo handler `show-notification`.
const NOTIFY_PERMISSION_JS: &str = r#"
(function () {
  try {
    Object.defineProperty(Notification, 'permission', {
      configurable: true,
      get: () => 'granted',
    });
    Notification.requestPermission = function (cb) {
      if (typeof cb === 'function') cb('granted');
      return Promise.resolve('granted');
    };
  } catch (e) {}
})();
"#;

/// Idiomas para a verificação ortográfica: os dicionários realmente instalados
/// no sistema, ordenados com os do locale do usuário primeiro. Assim funciona
/// com o que o usuário instalou (ex.: só pt_BR), mesmo que o locale seja outro.
fn spell_languages() -> Vec<String> {
    let available = available_dictionaries();
    if available.is_empty() {
        return Vec::new();
    }
    let mut ordered: Vec<String> = Vec::new();
    // Preferidos do locale, se estiverem instalados.
    for lang in locale_languages() {
        if available.iter().any(|a| *a == lang) && !ordered.contains(&lang) {
            ordered.push(lang);
        }
    }
    // Depois, o restante dos dicionários instalados.
    for lang in available {
        if !ordered.contains(&lang) {
            ordered.push(lang);
        }
    }
    ordered
}

/// Códigos de idioma dos dicionários hunspell/myspell instalados no sistema.
fn available_dictionaries() -> Vec<String> {
    let dirs = [
        "/usr/share/hunspell",
        "/usr/share/myspell/dicts",
        "/usr/local/share/hunspell",
    ];
    let mut langs: Vec<String> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("dic") {
                if let Some(lang) = path.file_stem().and_then(|s| s.to_str()) {
                    if !langs.iter().any(|l| l == lang) {
                        langs.push(lang.to_string());
                    }
                }
            }
        }
    }
    langs
}

/// Idiomas do locale do usuário (ex.: "pt_BR.UTF-8:en_US" -> ["pt_BR", "en_US"]).
fn locale_languages() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for var in ["LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(var) {
            for part in value.split(':') {
                let lang = part
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .split('@')
                    .next()
                    .unwrap_or("");
                if !lang.is_empty()
                    && lang != "C"
                    && lang != "POSIX"
                    && !out.iter().any(|l| l == lang)
                {
                    out.push(lang.to_string());
                }
            }
        }
    }
    out
}

/// Executa um script na webview (fire-and-forget).
fn run_js(webview: &webkit6::WebView, script: &str) {
    webview.evaluate_javascript(
        script,
        None,
        None,
        None::<&gtk::gio::Cancellable>,
        |_| {},
    );
}

/// Uma visão web isolada de um serviço, com sessão/cookies próprios.
///
/// Também é dona do seu próprio ícone, que mostra a inicial do serviço e é
/// trocado automaticamente pelo favicon do site assim que ele carrega — o
/// binding é conectado uma única vez, aqui.
#[derive(Clone)]
pub struct ServiceView {
    root: gtk::Widget,
    webview: webkit6::WebView,
    icon: ServiceIcon,
    muted: Rc<Cell<bool>>,
    home: String,
}

impl ServiceView {
    /// Cria a visão de um serviço apontando para `url`, com armazenamento
    /// persistente e isolado em `session_dir`. `app`/`dnd`/`muted` controlam
    /// o encaminhamento de notificações ao desktop.
    pub fn new(
        id: &str,
        name: &str,
        url: &str,
        session_dir: &Path,
        app: &adw::Application,
        dnd: Rc<Cell<bool>>,
        muted: bool,
    ) -> Self {
        let data = session_dir.join("data");
        let cache = session_dir.join("cache");
        let _ = std::fs::create_dir_all(&data);
        let _ = std::fs::create_dir_all(&cache);

        // Sessão de rede isolada: cada serviço tem seus próprios cookies e
        // armazenamento (equivalente às contas separadas do Franz).
        let session =
            webkit6::NetworkSession::new(data.to_str(), cache.to_str());
        if let Some(cookies) = session.cookie_manager() {
            if let Some(path) = data.join("cookies.sqlite").to_str() {
                cookies.set_persistent_storage(
                    path,
                    webkit6::CookiePersistentStorage::Sqlite,
                );
            }
        }
        // Habilita favicons para usá-los como ícone no rail lateral.
        if let Some(dm) = session.website_data_manager() {
            dm.set_favicons_enabled(true);
        }

        let settings = webkit6::Settings::new();
        settings.set_enable_developer_extras(true);
        settings.set_enable_smooth_scrolling(true);
        settings.set_media_playback_requires_user_gesture(false);
        // Alguns serviços checam a UA; a padrão do WebKit já funciona na maioria.

        // Handlers que recebem do JS: favicon rasterizado (PNG) e não lidas.
        let ucm = webkit6::UserContentManager::new();
        ucm.register_script_message_handler("faviconReady", None);
        ucm.register_script_message_handler("unreadCount", None);
        // Pré-concede a permissão de notificação (ver NOTIFY_PERMISSION_JS).
        ucm.add_script(&webkit6::UserScript::new(
            NOTIFY_PERMISSION_JS,
            webkit6::UserContentInjectedFrames::AllFrames,
            webkit6::UserScriptInjectionTime::Start,
            &[],
            &[],
        ));

        let webview = webkit6::WebView::builder()
            .network_session(&session)
            .settings(&settings)
            .user_content_manager(&ucm)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Verificação ortográfica usando os dicionários (enchant/hunspell) do
        // sistema, nos idiomas do locale do usuário.
        if let Some(context) = webview.context().or_else(webkit6::WebContext::default) {
            context.set_spell_checking_enabled(true);
            let langs = spell_languages();
            if !langs.is_empty() {
                let refs: Vec<&str> = langs.iter().map(String::as_str).collect();
                context.set_spell_checking_languages(&refs);
            }
        }

        // Concede permissões (notificações, mídia) automaticamente — é um
        // cliente dedicado de mensagens, então o comportamento esperado é
        // sempre permitir.
        webview.connect_permission_request(|_, request| {
            request.allow();
            true
        });

        // Encaminha as notificações do site para o desktop, respeitando o
        // silenciar do serviço e o "não perturbe" global.
        let muted = Rc::new(Cell::new(muted));
        {
            let app = app.clone();
            let dnd = dnd.clone();
            let muted = muted.clone();
            let id = id.to_string();
            let name = name.to_string();
            webview.connect_show_notification(move |_wv, notification| {
                if dnd.get() || muted.get() {
                    return true; // suprime (assumimos o controle da notificação)
                }
                let title = notification
                    .title()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.clone());
                let notif = gtk::gio::Notification::new(&title);
                if let Some(body) = notification.body() {
                    notif.set_body(Some(&body));
                }
                app.send_notification(Some(&id), &notif);
                true
            });
        }

        // "Nova janela"/popup (ex.: "Entrar com Google", target=_blank):
        // navega na PRÓPRIA webview do serviço, em vez de abrir popup ou o
        // navegador externo. Como é a mesma view, a sessão/cookies do login
        // ficam no serviço; o provedor OAuth redireciona de volta ao final.
        webview.connect_create(|wv, action| {
            if let Some(uri) = action.request().and_then(|r| r.uri()) {
                if !uri.is_empty() {
                    wv.load_uri(&uri);
                }
            }
            None::<gtk::Widget>
        });

        // Ícone do rail: inicial do nome como fallback, favicon quando disponível.
        let icon = ServiceIcon::new(name);
        // Caminho rápido: favicon raster que o WebKit já rastreia.
        icon.set_favicon(webview.favicon().as_ref());
        {
            let icon = icon.clone();
            webview.connect_favicon_notify(move |wv| {
                icon.set_favicon(wv.favicon().as_ref());
            });
        }
        // Caminho robusto: o JS rasteriza o ícone real (inclusive SVG) num
        // canvas 64px e posta o PNG aqui via message handler.
        {
            let icon = icon.clone();
            ucm.connect_script_message_received(Some("faviconReady"), move |_, value| {
                let data_url = value.to_str();
                if let Some(texture) = png_data_url_to_texture(&data_url) {
                    icon.set_favicon(Some(&texture));
                }
            });
        }
        // Não lidas: o JS observa o título/DOM e posta a contagem.
        {
            let icon = icon.clone();
            ucm.connect_script_message_received(Some("unreadCount"), move |_, value| {
                let n = value.to_int32().max(0) as u32;
                icon.set_badge(n);
            });
        }

        // Ao terminar de carregar, injeta os scripts de favicon e não lidas.
        webview.connect_load_changed(|wv, event| {
            if event == webkit6::LoadEvent::Finished {
                run_js(wv, FAVICON_JS);
                run_js(wv, UNREAD_JS);
            }
        });

        webview.load_uri(url);

        let root: gtk::Widget = webview.clone().upcast();
        Self {
            root,
            webview,
            icon,
            muted,
            home: url.to_string(),
        }
    }

    /// Silencia/dessilencia as notificações do serviço.
    pub fn set_muted(&self, muted: bool) {
        self.muted.set(muted);
    }

    /// Widget a ser inserido no `gtk::Stack` da janela.
    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }

    /// Widget do ícone do serviço para o rail lateral (favicon ou inicial).
    pub fn icon(&self) -> &gtk::Widget {
        self.icon.widget()
    }

    pub fn reload(&self) {
        self.webview.reload();
    }

    /// Volta para a URL inicial do serviço.
    pub fn go_home(&self) {
        self.webview.load_uri(&self.home);
    }
}

/// Converte um data URL "data:image/png;base64,…" em textura.
fn png_data_url_to_texture(data_url: &str) -> Option<gdk::Texture> {
    let b64 = data_url.strip_prefix("data:image/png;base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&bytes)).ok()
}
