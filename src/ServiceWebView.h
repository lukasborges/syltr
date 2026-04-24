#pragma once

#include "Service.h"

#include <QWebEngineView>

class QWebEngineProfile;
class QWebEnginePage;

class ServiceWebView : public QWebEngineView
{
    Q_OBJECT

public:
    explicit ServiceWebView(const Service &service, QWidget *parent = nullptr);

    const Service &service() const { return m_service; }

private:
    Service m_service;
    QWebEngineProfile *m_profile = nullptr;
    QWebEnginePage *m_page = nullptr;
};
