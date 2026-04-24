from __future__ import annotations

from PySide6.QtCore import QObject, Signal
from PySide6.QtGui import QAction, QIcon
from PySide6.QtWidgets import QMenu, QSystemTrayIcon


class TrayIcon(QObject):
    toggleRequested = Signal()
    quitRequested = Signal()

    def __init__(self, parent: QObject | None = None) -> None:
        super().__init__(parent)
        icon = QIcon.fromTheme("dev.syltr.Syltr", QIcon.fromTheme("internet-mail"))
        self._icon = QSystemTrayIcon(icon, self)
        self._icon.setToolTip("Syltr")

        menu = QMenu()
        show = QAction("Show / Hide", menu)
        show.triggered.connect(self.toggleRequested)
        quit_ = QAction("Quit", menu)
        quit_.triggered.connect(self.quitRequested)
        menu.addAction(show)
        menu.addSeparator()
        menu.addAction(quit_)
        self._icon.setContextMenu(menu)
        self._icon.activated.connect(self._on_activated)

    def show(self) -> None:
        if QSystemTrayIcon.isSystemTrayAvailable():
            self._icon.show()

    def _on_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        if reason == QSystemTrayIcon.ActivationReason.Trigger:
            self.toggleRequested.emit()
