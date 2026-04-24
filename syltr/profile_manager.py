from __future__ import annotations

from PySide6.QtCore import QObject, Slot
from PySide6.QtWebEngineCore import QWebEngineProfile, QWebEngineSettings

from .paths import profile_dir

_DESKTOP_UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/124.0.0.0 Safari/537.36"
)


class ProfileManager(QObject):
    def __init__(self, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self._profiles: dict[str, QWebEngineProfile] = {}

    @Slot(str, result=QWebEngineProfile)
    def profileFor(self, service_id: str) -> QWebEngineProfile:
        if service_id not in self._profiles:
            self._profiles[service_id] = self._build_profile(service_id)
        return self._profiles[service_id]

    def _build_profile(self, service_id: str) -> QWebEngineProfile:
        storage = profile_dir(service_id)
        profile = QWebEngineProfile(f"syltr-{service_id}", self)
        profile.setPersistentStoragePath(str(storage))
        profile.setCachePath(str(storage / "cache"))
        profile.setPersistentCookiesPolicy(
            QWebEngineProfile.PersistentCookiesPolicy.ForcePersistentCookies
        )
        profile.setHttpUserAgent(_DESKTOP_UA)

        s = profile.settings()
        s.setAttribute(QWebEngineSettings.WebAttribute.JavascriptEnabled, True)
        s.setAttribute(QWebEngineSettings.WebAttribute.LocalStorageEnabled, True)
        s.setAttribute(QWebEngineSettings.WebAttribute.ScrollAnimatorEnabled, True)
        s.setAttribute(QWebEngineSettings.WebAttribute.PlaybackRequiresUserGesture, False)
        s.setAttribute(QWebEngineSettings.WebAttribute.ShowScrollBars, False)
        return profile
