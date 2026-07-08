# Syltr

An all-in-one messaging service aggregator, native to **GNOME**. It gathers
WhatsApp Web, Telegram, Slack, Discord, Messenger and any other web service into
a single window — each with its own **isolated session** (separate cookies and
storage).

**Stack:** GTK4 · libadwaita · CEF (Chromium, offscreen) · Rust

## Features

- Icon rail with real favicons (SVG included) and an active-item highlight
- Add services from a catalog or by custom URL
- **Reorder** services by dragging · **context menu** (right click)
- **Unread badges** on the icon (detected from the page title)
- **Native desktop notifications** · **mute** per service · global **do not disturb**
- **Spell checking** with the system dictionaries · **camera/mic/calls** toggle
- **External links open in the default browser**; same-site and SSO navigations stay in-app
- Isolated session per service (independent login for each)
- Downloads saved straight to `~/Downloads` with a completion notification
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
./scripts/install-deps.sh          # toolchain, GTK4/libadwaita, spell check and CEF
./scripts/install-deps.sh --no-ide # skip GNOME Builder
```

CEF is not in the official repos. The most reliable way to get it is the
prebuilt binary (Spotify CEF builds); point the `cef` crate at it:

```bash
export CEF_PATH=/path/to/cef_binary_XXXX_linux64_minimal
```

Build and run:

```bash
cargo run
```

Useful environment variables:

- `CEF_PATH` — directory with the CEF resources (defaults to next to the binary)
- `SYLTR_DEBUG=1` — enable remote DevTools at `http://localhost:9222`
- `SYLTR_CEF_ARGS="..."` — extra Chromium switches, space-separated
- `SYLTR_LOCALE_DIR` — override the translations directory

### Tests

```bash
cargo test
```

Unit tests are colocated with the code they test: a module gains a `tests`
submodule declared as `#[cfg(test)] mod tests;`, with the tests living in a
sibling `tests.rs` inside the module folder (e.g. `src/engine/navigation/tests.rs`).
This keeps them out of the release build and gives them access to the module's
internal (`pub(crate)`/private) items.

## Architecture

The app talks to the web engine **only** through the public `engine::ServiceView`
API and never touches CEF directly. Each module is split into a folder by
responsibility:

| Path              | Responsibility                                              |
|-------------------|------------------------------------------------------------|
| `src/main.rs`     | Bootstrap CEF, then start the `AdwApplication` and CSS      |
| `src/window/`     | Window, rail, view stack, actions, dialogs, context menu    |
| `src/engine/`     | Web engine layer (CEF/OSR): bootstrap, render, handlers, `ServiceView` |
| `src/input/`      | Forwarding GTK input to CEF (mouse, scroll, keyboard, focus, IME) |
| `src/imgproxy/`   | Workaround for a CEF redirect bug on Google Chat images    |
| `src/config/`     | Service list, settings and their XDG file locations         |
| `src/spellcheck.rs` | Discovery of system spell-check dictionaries             |
| `src/catalog.rs`  | Catalog of known services ("recipes")                       |
| `src/icon.rs`     | The service icon (tile + favicon + unread badge)            |

### Web engine (CEF / OSR)

Each service is a **windowless CEF browser** rendering offscreen into a
`GtkDrawingArea` via Cairo, with an isolated session/cache per service. The rest
of the app depends only on `ServiceView` (`new`, `widget`, `icon`, `reload`,
`go_home`, `set_notifications_enabled`, `set_spell_languages`), so the engine
internals stay contained in `src/engine/`.

> On Wayland/Linux, CEF requires offscreen rendering and its binary distribution
> (`CEF_PATH`, see `scripts/install-deps.sh`).

## License

GPL-3.0-or-later
