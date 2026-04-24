from __future__ import annotations

from pathlib import Path

from PySide6.QtCore import QStandardPaths

from . import APP_ID

PACKAGE_ROOT = Path(__file__).resolve().parent
QML_DIR = PACKAGE_ROOT / "qml"
MAIN_QML = QML_DIR / "Main.qml"


def bundled_services_catalog() -> Path:
    return PACKAGE_ROOT / "resources" / "services.json"


def user_data_dir() -> Path:
    base = Path(QStandardPaths.writableLocation(QStandardPaths.StandardLocation.AppDataLocation))
    if not base.parts or base.name != APP_ID:
        base = base.parent / APP_ID if base.parts else Path.home() / ".local/share" / APP_ID
    base.mkdir(parents=True, exist_ok=True)
    return base


def user_services_catalog() -> Path:
    return user_data_dir() / "services.json"


def profile_dir(service_id: str) -> Path:
    p = user_data_dir() / "profiles" / service_id
    p.mkdir(parents=True, exist_ok=True)
    return p
