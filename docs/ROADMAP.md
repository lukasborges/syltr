# Roadmap

Milestones are ordered by dependency, not by time. Each milestone should ship a
user-visible improvement. No dates — this is a hobby project for now.

## 0.1 — Scaffold (current)

- [x] CMake project with Qt6 + KF6 + WebEngine.
- [x] `Service` value type + `ServiceManager` loading catalog from JSON
      (user override falls back to compiled-in `:/services.json`).
- [x] `ServiceWebView` subclass of `QWebEngineView` with an isolated
      `QWebEngineProfile` per service and persistent cookies/storage.
- [x] `MainWindow` (`KXmlGuiWindow`) with a `QListWidget` sidebar and
      `QStackedWidget` of views.
- [x] `TrayIcon` wrapping `KStatusNotifierItem` with show/hide/quit.
- [x] `.desktop` + AppStream metainfo.
- [x] Documentation + repo scaffolding.

**Exit criterion:** `./build/bin/syltr` opens a window, loads WhatsApp Web,
Telegram Web, Slack, Discord, Messenger; sessions persist across restarts.

## 0.2 — Real KDE integration

- [ ] Forward HTML5 `Notification` API through
      `QWebEngineProfile::setNotificationPresenter` into `KNotification` with
      per-service app identity.
- [ ] Unread badge: extract via per-service JS injected with
      `QWebEnginePage::runJavaScript`, surface on sidebar and tray tooltip.
- [ ] Global shortcut to show/hide via `KGlobalAccel`.
- [ ] Download handling: route to `~/Downloads`, wire through `KIO::JobUiDelegate`.
- [ ] Respect Plasma color scheme (`QPalette` + `KColorScheme`).

**Exit criterion:** Syltr feels like a Plasma-native app — notifications show
the service name, unread counts visible, Meta+M (configurable) toggles it.

## 0.3 — Service management UX

- [ ] In-app "Add/Remove service" dialog writes the user JSON; no manual edit
      needed for the common case.
- [ ] Drag-reorder sidebar; persist order.
- [ ] Per-service settings: mute notifications, sleep when hidden, custom UA.
- [ ] Icon resolver: fall back to favicon fetched via `QNetworkAccessManager`
      when no named theme icon matches.

**Exit criterion:** A new user can add Mattermost, Teams, or any self-hosted
web chat without touching a JSON file.

## 0.4 — Quality of life

- [ ] Session lock: spawn Syltr with `--lock` requiring a password to unhide
      (encrypts a per-install key, not the profiles themselves).
- [ ] "Do Not Disturb" toggle from tray.
- [ ] Adblock/privacy: integrate `qutebrowser`-style host blocklist via
      `QWebEngineUrlRequestInterceptor`.
- [ ] Spellcheck via bundled dictionaries.

## 0.5 — Packaging

- [ ] Flatpak manifest in `packaging/flatpak/` (KDE runtime).
- [ ] Fedora COPR spec (RPM) for system install.
- [ ] AppImage via `linuxdeploy` + `linuxdeploy-plugin-qt`.
- [ ] GitHub Actions: clang-format check, build matrix (Fedora + Debian),
      smoke boot test on headless Xvfb/offscreen.

**Exit criterion:** A non-developer can install Syltr from Flathub.

## 0.6 — Stretch

- [ ] Kirigami-based layout for Plasma Mobile form factor.
- [ ] Unified search across services (per-service JS probes).
- [ ] Per-service workspace (e.g., two Slack workspaces = two entries with
      separate profiles) — already possible via distinct `id`s, but add a
      dedicated "clone service" action.

## Explicitly out of scope

- Proprietary protocol clients (XMPP, Matrix, Signal). Syltr is a web wrapper,
  not a native client.
- Windows/macOS packaging. Community patches welcome, but CI focuses on Linux.
- E2EE key escrow, audit logging, compliance features.
