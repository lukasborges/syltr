# Architecture

Syltr is a thin Python host around a QML UI. The Python layer owns Qt lifetime,
persistent storage paths, per-service `QWebEngineProfile`s, and the service
catalog; QML owns layout and interaction.

## Process model

A single process, single `QApplication`. No IPC, no background services.
One `QQmlApplicationEngine` loads `syltr/qml/Main.qml`, which instantiates
one `WebEngineView` per enabled service inside a `StackLayout`.

Views are created eagerly on startup. This trades a slower cold start for
instant tab switching — the common case.

## Component responsibilities

### `syltr.app.SyltrApplication`
Bootstraps `QtWebEngineQuick`, creates the `QApplication`, wires context
properties (`serviceModel`, `paths`, `userAgent`, `appBridge`) into the QML
engine, loads `Main.qml`, and returns the Qt event loop exit code.

### `syltr.service_manager.ServiceListModel`
`QAbstractListModel` backed by a list of `Service` dataclasses. Loads services
from `~/.local/share/dev.syltr.Syltr/services.json` if it exists, otherwise
from the bundled catalog. Exposes custom roles (`serviceId`, `name`, `url`,
`icon`) to QML; no editing from QML yet.

### `syltr.paths.PathsHelper`
Tiny `QObject` with two slots (`profilePath(id)`, `cachePath(id)`) that return
per-service paths under `~/.local/share/dev.syltr.Syltr/profiles/<id>/`. Exposed
to QML as the `paths` context property.

Profiles themselves are created **inside QML** (`WebEngineProfile` elements in
`ServiceView.qml`), not in Python. Reason: PySide6 can't marshal
`QWebEngineProfile*` into QML's `QQuickWebEngineProfile*` slot type cleanly, so
we parametrize profile construction in QML using plain strings from Python.
Each profile is tied to its `WebEngineView` within the same delegate, keeping
lifetimes obvious.

### `syltr.tray.TrayIcon`
`QSystemTrayIcon` with a minimal menu (Show/Hide, Quit). On KDE Plasma this is
rendered by `StatusNotifierItem` transparently. Signals `toggleRequested` and
`quitRequested` are wired to the main window via `AppBridge`.

### QML

- **`Main.qml`** — `ApplicationWindow` with a sidebar + stack. Listens on
  `appBridge.toggleWindow` to show/hide from the tray.
- **`Sidebar.qml`** — `ListView` bound to `serviceModel`, emits `serviceSelected`
  with the row index.
- **`ServiceView.qml`** — `WebEngineView` bound to a profile obtained from
  `profileManager.profileFor(serviceId)`. Grants feature permissions (mic/cam/
  notifications) by default; this is intentional for a messaging client and will
  become per-service configurable later.

## Data flow

```
JSON on disk ─► ServiceListModel ─► QML Repeater ─► ServiceView[i]
                                                    ├─ WebEngineProfile (QML)
                                                    └─ WebEngineView
                                     paths.profilePath(id) ─────┘
                                     userAgent ─────────────────┘

Tray ─► AppBridge.toggleWindow ─► Main.qml Connections
```

## Persistence

- `~/.local/share/dev.syltr.Syltr/services.json` — user-overridable catalog.
- `~/.local/share/dev.syltr.Syltr/profiles/<id>/` — cookies, local storage,
  service workers, IndexedDB, Chromium cache per service.

The user's profiles directory is the only thing that must never be deleted
carelessly — losing it means re-logging into every service.

## Non-goals (for now)

- No plugin system. Services are JSON entries, not Electron-style recipes.
- No per-service preference UI. Edit JSON.
- No multi-window. One window, one stack.
- No mobile. Kirigami-based responsive layout is a future option, not a goal.
