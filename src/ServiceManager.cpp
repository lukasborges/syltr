#include "ServiceManager.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLoggingCategory>
#include <QStandardPaths>

Q_LOGGING_CATEGORY(lcServices, "syltr.services")

ServiceManager::ServiceManager(QObject *parent)
    : QObject(parent)
{
    reload();
}

void ServiceManager::reload()
{
    const QString userPath = QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
                                 .filePath(QStringLiteral("services.json"));
    if (QFile::exists(userPath) && loadFromFile(userPath)) {
        return;
    }
    loadFromFile(QStringLiteral(":/services.json"));
}

bool ServiceManager::loadFromFile(const QString &path)
{
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly)) {
        qCWarning(lcServices) << "Cannot open service catalog" << path;
        return false;
    }
    return loadFromJson(f.readAll());
}

bool ServiceManager::loadFromJson(const QByteArray &data)
{
    QJsonParseError err{};
    const auto doc = QJsonDocument::fromJson(data, &err);
    if (err.error != QJsonParseError::NoError || !doc.isArray()) {
        qCWarning(lcServices) << "Invalid service catalog:" << err.errorString();
        return false;
    }

    m_services.clear();
    const auto arr = doc.array();
    m_services.reserve(arr.size());
    for (const auto &v : arr) {
        const auto o = v.toObject();
        const QString id = o.value(QStringLiteral("id")).toString();
        const QString name = o.value(QStringLiteral("name")).toString();
        const QUrl url(o.value(QStringLiteral("url")).toString());
        if (id.isEmpty() || name.isEmpty() || !url.isValid()) {
            continue;
        }
        m_services.append(Service(id, name, url, o.value(QStringLiteral("icon")).toString()));
    }
    Q_EMIT servicesChanged();
    return true;
}
