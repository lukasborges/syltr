from __future__ import annotations

import sys

from .app import SyltrApplication


def main() -> int:
    app = SyltrApplication(sys.argv)
    return app.run()


if __name__ == "__main__":
    raise SystemExit(main())
