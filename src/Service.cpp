#include "Service.h"

#include <QDir>
#include <QStandardPaths>

Service::Service(QString id, QString name, QUrl url, QString iconName)
    : m_id(std::move(id))
    , m_name(std::move(name))
    , m_url(std::move(url))
    , m_iconName(std::move(iconName))
{
}

QString Service::profilePath() const
{
    const QString base = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    const QString path = QDir(base).filePath(QStringLiteral("profiles/") + m_id);
    QDir().mkpath(path);
    return path;
}

QString Service::cachePath() const
{
    return QDir(profilePath()).filePath(QStringLiteral("cache"));
}
