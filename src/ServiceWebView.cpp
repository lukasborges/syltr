#include "ServiceWebView.h"

#include <QWebEnginePage>
#include <QWebEngineProfile>
#include <QWebEngineSettings>

namespace
{
constexpr auto kUserAgent =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/124.0.0.0 Safari/537.36";
}

ServiceWebView::ServiceWebView(const Service &service, QWidget *parent)
    : QWebEngineView(parent)
    , m_service(service)
{
    m_profile = new QWebEngineProfile(QStringLiteral("syltr-") + service.id(), this);
    m_profile->setPersistentStoragePath(service.profilePath());
    m_profile->setCachePath(service.cachePath());
    m_profile->setPersistentCookiesPolicy(QWebEngineProfile::ForcePersistentCookies);
    m_profile->setHttpUserAgent(QString::fromLatin1(kUserAgent));

    auto *settings = m_profile->settings();
    settings->setAttribute(QWebEngineSettings::JavascriptEnabled, true);
    settings->setAttribute(QWebEngineSettings::LocalStorageEnabled, true);
    settings->setAttribute(QWebEngineSettings::ScrollAnimatorEnabled, true);
    settings->setAttribute(QWebEngineSettings::PlaybackRequiresUserGesture, false);
    settings->setAttribute(QWebEngineSettings::ShowScrollBars, false);

    m_page = new QWebEnginePage(m_profile, this);
    setPage(m_page);

    load(service.url());
}
