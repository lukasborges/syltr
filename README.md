# Syltr

An all-in-one messaging service aggregator, native to **GNOME**. It gathers
WhatsApp Web, Telegram, Slack, Discord, Messenger and any other web service into
a single window — each with its own **isolated session** (separate cookies and
storage).

**Stack:** GTK4 · libadwaita · WebKitGTK 6 · Rust

## Features

- Theme-aware welcome screen until a service is selected
- Icon rail with real, persistent favicons (SVG fallback included) and an active-item highlight
- Add services from a catalog or by custom URL
- **Reorder** services by dragging · **context menu** (right click)
- **Unread badges** on the icon (detected from the page title)
- **Native desktop notifications** · **mute** per service · global **do not disturb**
- **Spell checking** with the system dictionaries (enchant/hunspell)
- Links clicked in messages (`target=_blank`) open in the **default browser**; SSO popups and in-service navigation stay in-app, with back/forward
- Isolated session per service (independent login for each)
- Downloads saved straight to `~/Downloads` with a completion notification
- Camera, microphone and WebRTC call support
- Service list persisted in `~/.config/dev.syltr.Syltr/services.json`

### Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+1` … `Ctrl+9` | Go to service N |
| `Ctrl+PgDown` / `Alt+↓` | Next service |
| `Ctrl+PgUp` / `Alt+↑` | Previous service |
| `Ctrl+N` | Add service |
| `Ctrl+R` / `F5` | Reload |
| `Ctrl+Q` | Quit |

## Development

Install the dependencies (Arch Linux):

```bash
./scripts/install-deps.sh          # toolchain, GTK4/libadwaita, WebKitGTK, spell check
./scripts/install-deps.sh --no-ide # skip GNOME Builder
```

Build and run:

```bash
cargo run
```

Useful environment variables:

- `SYLTR_DEBUG=1` — forward the pages' JS errors/warnings to stderr
- `SYLTR_LOCALE_DIR` — override the translations directory
- `SYLTR_SW_RENDER=1` — disable hardware acceleration as a troubleshooting
  fallback (heavy web apps may use substantially more memory)
- `SYLTR_TEAMS_CALLS=1` — opt into experimental Teams calls; disabled by
  default because WebKitGTK's WebRTC/PipeWire path can hang Teams

### Media codecs (WhatsApp video)

WebKitGTK plays media through **GStreamer**, so H.264/AAC (WhatsApp videos)
work with the system codecs — on Arch, install `gst-plugins-good` and
`gst-libav`. No engine rebuild involved.

### Camera, microphone and calls

Media capture and WebRTC are enabled for all services. On Arch they require
`gst-plugin-pipewire`, `gst-plugins-bad` and `libnice`. WebKitGTK's PipeWire
device monitor is known to crash on some systems, so this support remains
experimental despite being enabled by default.

### Tests

```bash
cargo test
```

Unit tests are colocated with the code they test: a module gains a `tests`
submodule declared as `#[cfg(test)] mod tests;`, with the tests living in a
sibling `tests.rs` inside the module folder (e.g. `src/engine/unread/tests.rs`).
This keeps them out of the release build and gives them access to the module's
internal (`pub(crate)`/private) items.

## Architecture

The app talks to the web engine **only** through the public `engine::ServiceView`
API and never touches `webkit6` directly. Each module is split into a folder by
responsibility:

| Path              | Responsibility                                              |
|-------------------|------------------------------------------------------------|
| `src/main.rs`     | `AdwApplication` startup, i18n, CSS, embedded resources     |
| `src/window/`     | Window, rail, view stack, actions, dialogs, context menu    |
| `src/engine/`     | Web engine layer (WebKitGTK 6): sessions, favicons, unread, downloads, `ServiceView` |
| `src/config/`     | Service list, settings and their XDG file locations         |
| `src/spellcheck.rs` | Discovery of system spell-check dictionaries             |
| `src/catalog.rs`  | Catalog of known services ("recipes")                       |
| `src/icon.rs`     | The service icon (tile + favicon + unread badge)            |

### Web engine (WebKitGTK 6)

Each service is a `webkit6::WebView` with its own **`NetworkSession`**
(cookies, storage and cache isolated under the service's session directory).
The latest real favicon is cached alongside that session and remains available
while the service is suspended or disabled.
Only the selected service stays loaded by default. Background activity can be
enabled per service from **Edit service** to keep notifications and unread
badges updating; focused direct messengers use this mode by default, while
heavier workspaces, mail, calendar, tasks, AI and custom services do not.
The rest of the app depends only on `ServiceView` (`new`, `widget`, `icon`,
`reload`, `go_back`, `go_forward`, `go_home`, `set_notifications_enabled`,
`set_spell_languages`), so the engine internals stay contained in `src/engine/`.

Compatibility choices carried in the engine: hardware acceleration stays on
because heavy SPAs can exhaust memory on the software path, Google Calendar
uses a Safari user agent to avoid incompatible Chrome-specific code paths,
media capture/WebRTC stays enabled except for Teams calls (experimental opt-in)
because its PipeWire device-monitor path can hang, and a startup script shims
`Notification.permission` and `requestIdleCallback`.

## License

GPL-3.0-or-later
