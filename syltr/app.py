from __future__ import annotations

from collections.abc import Sequence

from PySide6.QtCore import QObject, QUrl, Signal
from PySide6.QtGui import QIcon
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtWebEngineQuick import QtWebEngineQuick
from PySide6.QtWidgets import QApplication

from . import APP_ID, __version__
from .paths import DESKTOP_USER_AGENT, MAIN_QML, PathsHelper
from .service_manager import ServiceListModel
from .tray import TrayIcon


class AppBridge(QObject):
    toggleWindow = Signal()


class SyltrApplication:
    def __init__(self, argv: Sequence[str]) -> None:
        QtWebEngineQuick.initialize()

        self._qapp = QApplication(list(argv))
        self._qapp.setApplicationName("Syltr")
        self._qapp.setApplicationDisplayName("Syltr")
        self._qapp.setApplicationVersion(__version__)
        self._qapp.setOrganizationName("Syltr")
        self._qapp.setOrganizationDomain("syltr.dev")
        self._qapp.setDesktopFileName(APP_ID)
        self._qapp.setWindowIcon(QIcon.fromTheme(APP_ID, QIcon.fromTheme("internet-mail")))
        self._qapp.setQuitOnLastWindowClosed(False)

        self._bridge = AppBridge()
        self._services = ServiceListModel()
        self._paths = PathsHelper()
        self._tray = TrayIcon()

        self._tray.toggleRequested.connect(self._bridge.toggleWindow)
        self._tray.quitRequested.connect(self._qapp.quit)

        self._engine = QQmlApplicationEngine()
        ctx = self._engine.rootContext()
        ctx.setContextProperty("serviceModel", self._services)
        ctx.setContextProperty("paths", self._paths)
        ctx.setContextProperty("userAgent", DESKTOP_USER_AGENT)
        ctx.setContextProperty("appBridge", self._bridge)

        self._engine.load(QUrl.fromLocalFile(str(MAIN_QML)))
        if not self._engine.rootObjects():
            raise RuntimeError(f"Failed to load QML from {MAIN_QML}")

        self._tray.show()

    def run(self) -> int:
        return self._qapp.exec()
