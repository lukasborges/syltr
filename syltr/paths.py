from __future__ import annotations

from pathlib import Path

from PySide6.QtCore import QObject, QStandardPaths, Slot

from . import APP_ID

PACKAGE_ROOT = Path(__file__).resolve().parent
QML_DIR = PACKAGE_ROOT / "qml"
MAIN_QML = QML_DIR / "Main.qml"

DESKTOP_USER_AGENT = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/124.0.0.0 Safari/537.36"
)


def bundled_services_catalog() -> Path:
    return PACKAGE_ROOT / "resources" / "services.json"


def user_data_dir() -> Path:
    base = Path(QStandardPaths.writableLocation(QStandardPaths.StandardLocation.AppDataLocation))
    if not base.parts:
        base = Path.home() / ".local/share" / APP_ID
    elif base.name != APP_ID:
        base = base.parent / APP_ID
    base.mkdir(parents=True, exist_ok=True)
    return base


def user_services_catalog() -> Path:
    return user_data_dir() / "services.json"


def profile_dir(service_id: str) -> Path:
    p = user_data_dir() / "profiles" / service_id
    p.mkdir(parents=True, exist_ok=True)
    return p


class PathsHelper(QObject):
    @Slot(str, result=str)
    def profilePath(self, service_id: str) -> str:
        return str(profile_dir(service_id))

    @Slot(str, result=str)
    def cachePath(self, service_id: str) -> str:
        return str(profile_dir(service_id) / "cache")
