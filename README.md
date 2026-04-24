# Syltr

All-in-one messaging client for KDE Plasma — a lightweight, native alternative
to Franz/Ferdium.

Syltr groups the web interfaces of common messaging services (WhatsApp Web,
Telegram, Slack, Discord, Messenger, and user-defined services) into a single
window, each with an isolated persistent session.

- **Status**: pre-alpha scaffold (0.1.0). Not usable yet.
- **Stack**: C++20, Qt 6.5+ (Widgets + WebEngine), KF6, CMake.
- **Target platform**: Linux with KDE Plasma 6.

## Why another messaging aggregator?

Franz and Ferdium are Electron-based. Syltr bets on the native Qt/KF6 stack
used by KDE itself: lighter footprint, Wayland-friendly, respects Plasma
theming, notifications, tray (`KStatusNotifierItem`), and global shortcuts.

## Build dependencies (Fedora)

```bash
sudo dnf install -y \
    cmake ninja-build gcc-c++ \
    extra-cmake-modules \
    qt6-qtbase-devel qt6-qtwebengine-devel qt6-qtsvg-devel \
    kf6-kcoreaddons-devel kf6-ki18n-devel kf6-kxmlgui-devel \
    kf6-kconfigwidgets-devel kf6-knotifications-devel \
    kf6-kstatusnotifieritem-devel kf6-kwidgetsaddons-devel \
    kf6-kcrash-devel kf6-kwindowsystem-devel
```

## Build

```bash
git clone https://github.com/syltr/syltr.git
cd syltr
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build build
./build/bin/syltr
```

## Install (optional)

```bash
cmake --install build --prefix ~/.local
```

The first run creates `~/.local/share/dev.syltr.Syltr/` with per-service
profile directories under `profiles/`.

## Configuring services

Services are declared in `resources/services.json` (compiled into the binary).
To override without rebuilding, drop a modified copy at
`~/.local/share/dev.syltr.Syltr/services.json`:

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
├── CMakeLists.txt
├── src/                  C++ sources (MainWindow, Service, ServiceManager, ServiceWebView, TrayIcon)
├── resources/            Qt resource bundle + bundled services.json
├── data/                 .desktop + AppStream metainfo
├── docs/                 Architecture, roadmap, contributing
├── LICENSE               GPL-3.0-or-later
└── README.md
```

## Development

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the internal design,
[`docs/ROADMAP.md`](docs/ROADMAP.md) for the planned milestones, and
[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) for code style and PR flow.

If you're coming from Java and Qt/C++ is new to you, start with
[`docs/FROM_JAVA.md`](docs/FROM_JAVA.md) — it maps Qt idioms to Swing/Java
equivalents and walks through the Syltr codebase in that lens.

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
