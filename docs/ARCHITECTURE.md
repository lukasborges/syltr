# Architecture

Syltr is a native Qt/KF6 application. One process, one `QApplication`, one
`MainWindow`. No IPC, no background daemons.

## Component responsibilities

### `main.cpp`
Sets up `KAboutData`, `KCrash`, and the application-wide icon/desktop identity.
Parses command-line arguments via `KAboutData::setupCommandLine`. Creates the
`MainWindow` and starts the Qt event loop.

### `Service` (value type)
Plain data class holding `id`, `name`, `url`, and `iconName`. Computes its own
per-service `profilePath()` / `cachePath()` under
`~/.local/share/dev.syltr.Syltr/profiles/<id>/`. No Qt meta-object — this is a
regular copyable C++ value.

### `ServiceManager` (`QObject`)
Loads the service catalog. Prefers
`~/.local/share/dev.syltr.Syltr/services.json` if present, otherwise falls
back to the bundled `:/services.json` compiled from `resources/services.json`.
Emits `servicesChanged()` on reload.

### `ServiceWebView` (extends `QWebEngineView`)
One per service. Owns a `QWebEngineProfile` with:
- `storageName: "syltr-<id>"` so each service has isolated cookies/local storage.
- `persistentStoragePath` and `cachePath` under the service's profile directory.
- Desktop Chrome User-Agent so services don't serve us their mobile UI.
- `ForcePersistentCookies` so logins survive restarts.

The `QWebEngineProfile` is parented to the view, so it dies with the view.
This is the Qt equivalent of Java's parent-child object lifecycle.

### `TrayIcon` (`QObject`)
Thin wrapper around `KStatusNotifierItem` — the KDE-native tray implementation
(StatusNotifierItem spec). Emits `toggleRequested()` when the user clicks the
icon or the "Show / Hide" menu entry.

### `MainWindow` (extends `KXmlGuiWindow`)
- Holds the `ServiceManager`, `TrayIcon`, a `QListWidget` sidebar, and a
  `QStackedWidget` of `ServiceWebView`s.
- Reacts to `ServiceManager::servicesChanged` by rebuilding both the sidebar
  and the stack in `rebuildServiceViews()`.
- Sidebar row change → `QStackedWidget::setCurrentIndex`.
- `closeEvent` hides instead of quitting — the tray icon is the only way to
  fully exit (`File → Quit` or the tray menu).

## Data flow

```
resources/services.json (or ~/.local/share/…/services.json)
          │
          ▼
   ServiceManager ──── servicesChanged ────► MainWindow
          │                                       │
          │                                       ├─ QListWidget (sidebar)
          │                                       └─ QStackedWidget ─► ServiceWebView[i]
          │                                                                   │
          │                                                                   └─ QWebEngineProfile (isolated)

KStatusNotifierItem ── toggleRequested ─► MainWindow::toggleVisible
```

## Memory model

Every class except `Service` is a `QObject` with a parent. Objects are deleted
when their parent is deleted, so the codebase uses raw `new` without
smart pointers — this is idiomatic Qt and in practice behaves like Java's GC
for UI-owned objects. Only use `std::unique_ptr`/`std::shared_ptr` for
non-`QObject` data that outlives simple scoping.

## Persistence

- `~/.local/share/dev.syltr.Syltr/services.json` — user-overridable catalog.
- `~/.local/share/dev.syltr.Syltr/profiles/<id>/` — cookies, local storage,
  service workers, IndexedDB, Chromium cache per service.

The profiles directory is the only thing that must never be deleted casually —
losing it means re-logging into every service.

## Non-goals (for now)

- No plugin system. Services are JSON entries, not Franz-style recipes.
- No per-service preference UI. Edit JSON.
- No multi-window. One window, one stack.
- No mobile form factor.
