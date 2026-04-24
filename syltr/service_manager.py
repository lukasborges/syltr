from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from PySide6.QtCore import (
    QAbstractListModel,
    QByteArray,
    QModelIndex,
    Qt,
    Signal,
    Slot,
)

from .paths import bundled_services_catalog, user_services_catalog


@dataclass(frozen=True, slots=True)
class Service:
    id: str
    name: str
    url: str
    icon: str


class ServiceListModel(QAbstractListModel):
    IdRole = Qt.ItemDataRole.UserRole + 1
    NameRole = Qt.ItemDataRole.UserRole + 2
    UrlRole = Qt.ItemDataRole.UserRole + 3
    IconRole = Qt.ItemDataRole.UserRole + 4

    servicesChanged = Signal()

    def __init__(self) -> None:
        super().__init__()
        self._services: list[Service] = []
        self.reload()

    def reload(self) -> None:
        catalog = user_services_catalog()
        if not catalog.exists():
            catalog = bundled_services_catalog()
        self._load(catalog)

    def _load(self, path: Path) -> None:
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            raw = []
        self.beginResetModel()
        self._services = [
            Service(id=e["id"], name=e["name"], url=e["url"], icon=e.get("icon", ""))
            for e in raw
            if {"id", "name", "url"} <= e.keys()
        ]
        self.endResetModel()
        self.servicesChanged.emit()

    def rowCount(self, parent: QModelIndex = QModelIndex()) -> int:  # noqa: B008
        return 0 if parent.isValid() else len(self._services)

    def data(self, index: QModelIndex, role: int = Qt.ItemDataRole.DisplayRole):
        if not index.isValid() or not (0 <= index.row() < len(self._services)):
            return None
        s = self._services[index.row()]
        if role in (Qt.ItemDataRole.DisplayRole, self.NameRole):
            return s.name
        if role == self.IdRole:
            return s.id
        if role == self.UrlRole:
            return s.url
        if role == self.IconRole:
            return s.icon
        return None

    def roleNames(self) -> dict[int, QByteArray]:
        return {
            self.IdRole: QByteArray(b"serviceId"),
            self.NameRole: QByteArray(b"name"),
            self.UrlRole: QByteArray(b"url"),
            self.IconRole: QByteArray(b"icon"),
        }

    @Slot(int, result="QVariantMap")
    def get(self, row: int) -> dict[str, str]:
        if not (0 <= row < len(self._services)):
            return {}
        s = self._services[row]
        return {"id": s.id, "name": s.name, "url": s.url, "icon": s.icon}
