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
/// 1) replaces the Web Notification API with a shim that forwards notifications
///    to the native side via the `syltrNotify` handler. WebKitGTK only grants
///    the real permission from a user gesture and never persists it for the
///    isolated per-service sessions, so `new Notification()` silently failed;
///    the shim reports permission as granted via both the Notification and
///    Permissions APIs, and also forwards `showNotification()` calls used by
///    service-worker based apps.
/// 2) polyfills requestIdleCallback/cancelIdleCallback for pages where the
///    native feature flag doesn't take (Microsoft Teams breaks without it).
pub(super) const COMPAT_JS: &str = r#"
(function () {
  try {
    let seq = 0;
    const post = (data) => {
      try {
        window.webkit.messageHandlers.syltrNotify.postMessage(JSON.stringify(data));
        return true;
      } catch (e) {
        return false;
      }
    };
    const dispatchNotification = (title, options) => {
      options = options || {};
      return post({
        id: ++seq,
        title: String(title == null ? '' : title),
        body: options.body || '',
        tag: options.tag || '',
      });
    };
    class SyltrNotification extends EventTarget {
      constructor(title, options) {
        super();
        options = options || {};
        this.title = String(title == null ? '' : title);
        this.body = options.body || '';
        this.icon = options.icon || '';
        this.tag = options.tag || '';
        this.data = options.data;
        this.dir = options.dir || 'auto';
        this.lang = options.lang || '';
        this.silent = !!options.silent;
        this.onclick = null;
        this.onshow = null;
        this.onclose = null;
        this.onerror = null;
        const ok = dispatchNotification(this.title, options);
        Promise.resolve().then(() => {
          const ev = new Event(ok ? 'show' : 'error');
          this.dispatchEvent(ev);
          const h = ok ? this.onshow : this.onerror;
          if (typeof h === 'function') { try { h.call(this, ev); } catch (e) {} }
        });
      }
      close() {
        const ev = new Event('close');
        this.dispatchEvent(ev);
        if (typeof this.onclose === 'function') { try { this.onclose(ev); } catch (e) {} }
      }
    }
    Object.defineProperty(SyltrNotification, 'permission', { get: () => 'granted' });
    Object.defineProperty(SyltrNotification, 'maxActions', { get: () => 0 });
    SyltrNotification.requestPermission = function (cb) {
      if (typeof cb === 'function') cb('granted');
      return Promise.resolve('granted');
    };
    window.Notification = SyltrNotification;

    if (navigator.permissions && typeof navigator.permissions.query === 'function') {
      const query = navigator.permissions.query.bind(navigator.permissions);
      navigator.permissions.query = function (descriptor) {
        const name = descriptor && String(descriptor.name || '').toLowerCase();
        if (name === 'notifications') {
          const status = new EventTarget();
          Object.defineProperty(status, 'name', { get: () => 'notifications' });
          Object.defineProperty(status, 'state', { get: () => 'granted' });
          status.onchange = null;
          return Promise.resolve(status);
        }
        return query(descriptor);
      };
    }

    if (window.ServiceWorkerRegistration && ServiceWorkerRegistration.prototype) {
      ServiceWorkerRegistration.prototype.showNotification = function (title, options) {
        dispatchNotification(title, options);
        return Promise.resolve();
      };
      ServiceWorkerRegistration.prototype.getNotifications = function () {
        return Promise.resolve([]);
      };
    }
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

/// WebKitGTK can emit a paste event without image files even though the GTK
/// clipboard contains a texture. Ask the native side for that texture only
/// when WebKit did not provide an image, then redispatch it as a PNG File.
pub(super) const CLIPBOARD_BRIDGE_JS: &str = r#"
(function () {
  if (window.__syltrClipboardBridge) return;
  window.__syltrClipboardBridge = true;

  window.__syltrPasteImage = function (base64) {
    try {
      const binary = atob(base64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
      const file = new File([bytes], 'clipboard.png', { type: 'image/png' });
      const transfer = new DataTransfer();
      transfer.items.add(file);
      const event = new ClipboardEvent('paste', {
        clipboardData: transfer,
        bubbles: true,
        cancelable: true,
        composed: true,
      });
      // Some WebKit versions ignore clipboardData in the constructor.
      if (!event.clipboardData || event.clipboardData.files.length === 0) {
        Object.defineProperty(event, 'clipboardData', { value: transfer });
      }
      (document.activeElement || document.body || document).dispatchEvent(event);
    } catch (e) {
      console.warn('[syltr] could not inject clipboard image', e);
    }
  };

  document.addEventListener('paste', function (event) {
    const data = event.clipboardData;
    const items = data ? Array.from(data.items || []) : [];
    const files = data ? Array.from(data.files || []) : [];
    const hasImage = items.some((item) => String(item.type).startsWith('image/'))
      || files.some((file) => String(file.type).startsWith('image/'));
    if (hasImage) return;
    try { window.webkit.messageHandlers.syltrPasteImage.postMessage('paste'); } catch (e) {}
  }, true);
})();
"#;

/// Workaround for a WebKitGTK bug (2.52): the media pipeline corrupts `blob:`
/// sources larger than 2 MB (framing drift under the WebKitWebSrc download
/// stop/restart cycle), which freezes e.g. WhatsApp videos. Media blobs are
/// re-served as `data:` URLs, whose delivery path is unaffected. MSE object
/// URLs and oversized blobs fall through to the original source.
pub(super) const BLOB_MEDIA_JS: &str = r#"
(function () {
  const MAX_BYTES = 64 * 1024 * 1024;
  const blobs = new Map();
  const inFlight = new Map();
  const createURL = URL.createObjectURL.bind(URL);
  const revokeURL = URL.revokeObjectURL.bind(URL);

  URL.createObjectURL = function (obj) {
    const url = createURL(obj);
    if (obj instanceof Blob && obj.size <= MAX_BYTES) blobs.set(url, obj);
    return url;
  };
  URL.revokeObjectURL = function (url) {
    blobs.delete(url);
    const pending = inFlight.get(url);
    if (pending) pending.finally(() => revokeURL(url));
    else revokeURL(url);
  };

  const asDataUrl = (blob) => new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });

    const desc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
  if (!desc || !desc.set) return;
  Object.defineProperty(HTMLMediaElement.prototype, 'src', {
    configurable: true,
    enumerable: desc.enumerable,
    get: desc.get,
    set(value) {
      const url = String(value);
      if (!url.startsWith('blob:')) return desc.set.call(this, value);
      const el = this;
      if (el.tagName === 'AUDIO') return desc.set.call(this, value);
      const known = blobs.get(url);
      const conversion = (known ? asDataUrl(known)
        : fetch(url).then((r) => r.blob()).then((b) => {
            if (b.size > MAX_BYTES) throw new Error('blob too large');
            return asDataUrl(b);
          }))
        .then((dataUrl) => {
          const resyncAudio = () => {
            const muted = el.muted;
            el.muted = !muted;
            el.muted = muted;
            const volume = el.volume;
            el.volume = volume > 0.5 ? volume - 0.05 : volume + 0.05;
            el.volume = volume;
          };
          el.addEventListener('playing', resyncAudio, { once: true });
          el.addEventListener('timeupdate', resyncAudio, { once: true });
          desc.set.call(el, dataUrl);
        })
        .catch(() => desc.set.call(el, url));
      inFlight.set(url, conversion);
      conversion.finally(() => inFlight.delete(url));
    },
  });

  const setAttr = HTMLMediaElement.prototype.setAttribute;
  HTMLMediaElement.prototype.setAttribute = function (name, value) {
    if (String(name).toLowerCase() === 'src') {
      this.src = value;
      return;
    }
    return setAttr.call(this, name, value);
  };
})();
"#;

/// Prevents web apps from publishing transport controls through Media Session.
/// WebKitGTK 2.52 skips MPRIS registration when native media metadata has no
/// title, so install empty metadata before the page can provide its own. Media
/// playback remains untouched; routing WhatsApp through Web Audio can leave it
/// silent when WebKitGTK keeps the AudioContext suspended.
pub(super) const SUPPRESS_MPRIS_JS: &str = r#"
(function () {
  if (window.__syltrMprisGuard) return;
  window.__syltrMprisGuard = true;

  const nope = function () {};

  try {
    const ms = navigator.mediaSession;
    const proto = window.MediaSession && window.MediaSession.prototype;
    const metadataDescriptor = proto
      && Object.getOwnPropertyDescriptor(proto, 'metadata');
    let emptyMetadata = null;
    if (ms && metadataDescriptor && metadataDescriptor.set && window.MediaMetadata) {
      emptyMetadata = new window.MediaMetadata({ title: '' });
      metadataDescriptor.set.call(ms, emptyMetadata);
    }
    if (ms) {
      try { ms.setActionHandler = nope; } catch (e) {}
      try { ms.setPositionState = nope; } catch (e) {}
    }
    if (proto) {
      try { proto.setActionHandler = nope; } catch (e) {}
      try { proto.setPositionState = nope; } catch (e) {}
      try {
        Object.defineProperty(proto, 'metadata', {
          configurable: true,
          get: () => emptyMetadata,
          set: nope,
        });
      } catch (e) {}
      try {
        Object.defineProperty(proto, 'playbackState', {
          configurable: true,
          get: () => 'none',
          set: nope,
        });
      } catch (e) {}
    }
  } catch (e) {}

})();
"#;

/// Runs a script on the webview (fire-and-forget).
pub(super) fn run_js(webview: &webkit6::WebView, script: &str) {
    webview.evaluate_javascript(script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
}
