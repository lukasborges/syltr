#pragma once

#include "Service.h"

#include <QObject>
#include <QVector>

class ServiceManager : public QObject
{
    Q_OBJECT

public:
    explicit ServiceManager(QObject *parent = nullptr);

    const QVector<Service> &services() const { return m_services; }

    void reload();

Q_SIGNALS:
    void servicesChanged();

private:
    bool loadFromFile(const QString &path);
    bool loadFromJson(const QByteArray &data);

    QVector<Service> m_services;
};
