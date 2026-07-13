//! JS snippets injected into the pages and the helper to run them.

use webkit6::prelude::*;

/// Injected when a page finishes loading: finds the best icon declared by the
/// page, rasterizes it on a canvas (WebKit renders any format, SVG included)
/// and posts the PNG data URL back via the `faviconReady` message handler.
pub(super) const FAVICON_JS: &str = r#"
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
        // Never upscale raster icons (blurry); SVG rasterizes at high resolution.
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

/// Injected at the start of every page (compatibility):
/// 1) makes the site see the notification permission as granted (WebKit does
///    not persist the permission, so WhatsApp kept showing its banner);
/// 2) polyfills requestIdleCallback/cancelIdleCallback for pages where the
///    native feature flag doesn't take (Microsoft Teams breaks without it).
pub(super) const COMPAT_JS: &str = r#"
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
      // Real-ish idle: ~50ms delay (honors options.timeout) so apps that
      // reschedule idle callbacks in a loop don't flood the event loop.
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

/// JS error capture (enabled by SYLTR_DEBUG): forwards console.error/warn,
/// window.onerror and rejected promises to the `consoleCapture` handler.
pub(super) const CONSOLE_JS: &str = r#"
(function () {
  const post = (kind, msg) => {
    try { window.webkit.messageHandlers.consoleCapture.postMessage(kind + ': ' + msg); } catch (e) {}
  };
  const fmt = (args) => Array.from(args).map((a) => {
    try { return typeof a === 'object' ? JSON.stringify(a) : String(a); } catch (e) { return String(a); }
  }).join(' ');
  ['error', 'warn'].forEach((name) => {
    const orig = console[name];
    console[name] = function () { post(name, fmt(arguments)); return orig.apply(console, arguments); };
  });
  window.addEventListener('error', (e) =>
    post('onerror', (e.message || '') + ' @ ' + (e.filename || '') + ':' + (e.lineno || '')));
  window.addEventListener('unhandledrejection', (e) =>
    post('unhandledrejection', String((e.reason && (e.reason.stack || e.reason.message)) || e.reason)));
})();
"#;

/// Runs a script on the webview (fire-and-forget).
pub(super) fn run_js(webview: &webkit6::WebView, script: &str) {
    webview.evaluate_javascript(script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
}
