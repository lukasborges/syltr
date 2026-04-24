# Contributing

Thanks for considering a contribution to Syltr.

## Getting started

```bash
git clone https://github.com/syltr/syltr.git
cd syltr
python -m venv .venv && source .venv/bin/activate
pip install -e '.[dev]'
python -m syltr
```

## Code style

- **Python**: formatted and linted with `ruff`, type-checked with `mypy`.
  Before opening a PR:
  ```bash
  ruff format .
  ruff check .
  mypy syltr
  ```
- **QML**: 4-space indentation, components in PascalCase files, one component
  per file, logic in JS inline only for small glue — anything non-trivial goes
  to Python.
- **Imports** are sorted; prefer explicit relative imports inside the `syltr`
  package.
- Public classes and signals exposed to QML should keep Qt-style camelCase
  names (e.g. `toggleRequested`); internal helpers stay snake_case.

## Commits and PRs

- One logical change per PR. Prefer small, reviewable diffs.
- Commit subject in imperative mood, ≤ 72 chars. Body explains *why*, not *what*.
- Reference an issue when one exists: `Fixes #123`.
- PRs should update `docs/ROADMAP.md` when they close a roadmap item.

## Adding a service

Services are data, not code. Open a PR editing `syltr/resources/services.json`
with:
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
- Python version (`python --version`).
- Output of `pip show PySide6 | grep Version`.
- Steps to reproduce and expected vs. actual behavior.

## Licensing

By contributing you agree that your work is licensed under GPL-3.0-or-later,
the same license as the rest of the project.
