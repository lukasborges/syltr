#pragma once

#include <QString>
#include <QUrl>

class Service
{
public:
    Service() = default;
    Service(QString id, QString name, QUrl url, QString iconName);

    QString id() const { return m_id; }
    QString name() const { return m_name; }
    QUrl url() const { return m_url; }
    QString iconName() const { return m_iconName; }

    QString profilePath() const;
    QString cachePath() const;

private:
    QString m_id;
    QString m_name;
    QUrl m_url;
    QString m_iconName;
};
