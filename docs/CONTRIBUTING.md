# Contributing

Thanks for considering a contribution to Syltr.

## Getting started

Install build dependencies (Fedora):

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

Configure and build:

```bash
git clone https://github.com/syltr/syltr.git
cd syltr
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
cmake --build build
./build/bin/syltr
```

`compile_commands.json` in `build/` lets clangd and IDEs (KDevelop, Qt Creator,
CLion) provide full code intelligence.

## Code style

- **Language**: C++20.
- **Formatter**: `clang-format` with the repo's `.clang-format` (WebKit-based).
  Run it on touched files before committing: `clang-format -i src/Changed.cpp`.
- **Naming**:
  - Classes: `PascalCase` (`ServiceManager`).
  - Member variables: `m_camelCase` (`m_sidebar`).
  - Methods and local variables: `camelCase`.
  - Qt signals: `camelCase` past-tense when describing events
    (`servicesChanged`) or action-like when describing intent
    (`toggleRequested`).
- **Includes**: project headers first (double quotes), then a blank line,
  then Qt (`<QFoo>`), then KF6 (`<KFoo>`), then standard library. `clang-format`
  with `SortIncludes: CaseSensitive` keeps each block sorted.
- **Memory**: every `QObject` subclass is created with `new` and a parent.
  Don't use `std::unique_ptr<QWidget>` — the parent-child system already owns it.
- **No raw UI strings**: wrap user-facing text in `i18n(...)` (from
  `KLocalizedString`). Log strings stay untranslated.
- **Error handling**: prefer `bool` return + `qCWarning(category)` logs for
  recoverable errors. No exceptions in Qt code.

## Commits and PRs

- One logical change per PR. Prefer small, reviewable diffs.
- Commit subject in imperative mood, ≤ 72 chars. Body explains *why*, not
  *what*.
- Reference an issue when one exists: `Fixes #123`.
- PRs should update `docs/ROADMAP.md` when they close a roadmap item.

## Adding a service

Services are data, not code. Open a PR editing `resources/services.json` with:
- A unique `id` (lowercase, ASCII, no spaces).
- A human-readable `name`.
- An `https://` URL pointing at the web client.
- An `icon` hint (freedesktop icon-theme name if available, else empty string).

Services that require workarounds (injecting CSS, overriding the UA, etc.)
aren't a good fit until Syltr grows a per-service recipe system (see
`docs/ROADMAP.md`).

## Reporting bugs

Please include:
- Distro + Plasma version.
- Qt version: `qmake6 --version` or `rpm -q qt6-qtbase`.
- Steps to reproduce and expected vs. actual behavior.
- If it's a crash, the output of `coredumpctl info syltr` after the crash.

## Licensing

By contributing you agree that your work is licensed under GPL-3.0-or-later,
the same license as the rest of the project.
