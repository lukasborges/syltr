#pragma once

#include <KXmlGuiWindow>

#include <QHash>

class QListWidget;
class QListWidgetItem;
class QStackedWidget;
class ServiceManager;
class ServiceWebView;
class TrayIcon;

class MainWindow : public KXmlGuiWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);

protected:
    void closeEvent(QCloseEvent *event) override;

private Q_SLOTS:
    void rebuildServiceViews();
    void onServiceSelected(int row);
    void toggleVisible();

private:
    void setupActions();
    void setupUi();

    ServiceManager *m_services = nullptr;
    TrayIcon *m_tray = nullptr;
    QListWidget *m_sidebar = nullptr;
    QStackedWidget *m_stack = nullptr;
    QHash<QString, ServiceWebView *> m_views;
};
