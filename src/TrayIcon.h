#pragma once

#include <QObject>

class KStatusNotifierItem;

class TrayIcon : public QObject
{
    Q_OBJECT

public:
    explicit TrayIcon(QObject *parent = nullptr);

Q_SIGNALS:
    void toggleRequested();

private:
    KStatusNotifierItem *m_item = nullptr;
};
