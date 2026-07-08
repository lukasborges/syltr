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
        const nat = Math.max(img.naturalWidth || 0, img.naturalHeight || 0) || 64;
        const isSvg = /\.svg(\?|$)/i.test(img.src);
        // Não amplia raster (evita borrão); SVG rasteriza em alta resolução.
        const target = isSvg ? 128 : Math.min(nat, 128);
        const c = document.createElement('canvas');
        c.width = target; c.height = target;
        const ctx = c.getContext('2d');
        ctx.clearRect(0, 0, target, target);
        ctx.drawImage(img, 0, 0, target, target);
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

/// Script injetado no início de cada página (compatibilidade):
/// 1) faz o site enxergar a permissão de notificação como concedida (o WebKit
///    não persiste a permissão, então o WhatsApp mostrava o banner sempre);
/// 2) faz polyfill de requestIdleCallback/cancelIdleCallback, que o WebKitGTK
///    não expõe e que serviços como o Microsoft Teams exigem (senão quebram
///    com "Can't find variable: requestIdleCallback").
const COMPAT_JS: &str = r#"
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
  try {
    if (typeof window.requestIdleCallback !== 'function') {
      // Idle "de verdade": atraso ~50ms (respeita options.timeout) para não
      // inundar o event loop quando o app reagenda idle callbacks em loop.
      window.requestIdleCallback = function (cb, opts) {
        var t = (opts && opts.timeout) ? Math.min(opts.timeout, 100) : 50;
        return setTimeout(function () {
          var start = Date.now();
          cb({
            didTimeout: false,
            timeRemaining: function () { return Math.max(0, 16 - (Date.now() - start)); },
          });
        }, t);
      };
      window.cancelIdleCallback = function (id) { clearTimeout(id); };
    }
  } catch (e) {}
})();
"#;

/// Aplica a verificação ortográfica no contexto da webview, nos idiomas dados
/// (lista vazia desliga).
fn apply_spell(webview: &webkit6::WebView, langs: &[String]) {
    if let Some(context) = webview.context().or_else(webkit6::WebContext::default) {
        context.set_spell_checking_enabled(!langs.is_empty());
        if !langs.is_empty() {
            let refs: Vec<&str> = langs.iter().map(String::as_str).collect();
            context.set_spell_checking_languages(&refs);
        }
    }
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
        spell_langs: &[String],
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
        // Shims de compatibilidade no início da página (ver COMPAT_JS).
        ucm.add_script(&webkit6::UserScript::new(
            COMPAT_JS,
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

        // Verificação ortográfica (backend enchant/hunspell), nos idiomas
        // escolhidos. Aplicada ao contexto compartilhado das webviews.
        apply_spell(&webview, spell_langs);

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

    /// Atualiza os idiomas da verificação ortográfica (lista vazia desliga).
    pub fn set_spell_languages(&self, langs: &[String]) {
        apply_spell(&self.webview, langs);
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
