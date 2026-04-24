# Syltr

All-in-one messaging client for KDE Plasma — a lightweight, native-feeling
alternative to Franz/Ferdium.

Syltr groups the web interfaces of common messaging services (WhatsApp Web,
Telegram, Slack, Discord, Messenger, and user-defined services) into a single
window, each with an isolated persistent session.

- **Status**: pre-alpha scaffold (0.1.0). Not usable yet.
- **Stack**: Python 3.11+, PySide6 (Qt 6.5+), QML, Qt WebEngine.
- **Target platform**: Linux with KDE Plasma 6. Should run on other Qt-capable
  desktops, but KDE is the reference environment.

## Why another messaging aggregator?

Franz and Ferdium are Electron-based. Syltr bets on the native Qt stack used by
KDE itself: lighter footprint, Wayland-friendly, respects Plasma theming,
notifications, tray, and global shortcuts.

## Prerequisites

Runtime dependencies are installed via `pip`. At the system level you need:

- Python **3.11** or later
- Qt 6.5+ system libraries (installed automatically by `PySide6` wheels)

On Fedora, additional system bits that help with native integration:

```bash
sudo dnf install python3-pip qt6-qtwebengine
```

No Qt or KF6 development packages are required — PySide6 ships its own Qt.

## Quick start

```bash
git clone https://github.com/syltr/syltr.git
cd syltr
python -m venv .venv && source .venv/bin/activate
pip install -e '.[dev]'
python -m syltr
```

The first run creates `~/.local/share/dev.syltr.Syltr/` and per-service profile
directories under `profiles/`.

## Configuring services

Services are declared in `syltr/resources/services.json` (bundled). To override,
copy it to `~/.local/share/dev.syltr.Syltr/services.json` and edit:

```json
[
    { "id": "whatsapp", "name": "WhatsApp Web", "url": "https://web.whatsapp.com/", "icon": "whatsapp" },
    { "id": "my-mattermost", "name": "Work Chat", "url": "https://chat.example.com/", "icon": "" }
]
```

Restart Syltr to reload.

## Repository layout

```
syltr/
├── syltr/              Python package
│   ├── app.py            QApplication + QML engine bootstrap
│   ├── service_manager.py  QAbstractListModel exposing services to QML
│   ├── profile_manager.py  Per-service QWebEngineProfile factory
│   ├── tray.py             QSystemTrayIcon wrapper
│   ├── paths.py            XDG paths
│   ├── qml/                QML UI (Main, Sidebar, ServiceView)
│   └── resources/          Bundled services.json
├── data/               .desktop + AppStream metainfo
├── docs/               Architecture, roadmap, contributing
├── pyproject.toml
├── LICENSE             GPL-3.0-or-later
└── README.md
```

## Development

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the internal design,
[`docs/ROADMAP.md`](docs/ROADMAP.md) for the planned milestones, and
[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) for code style and PR flow.

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
