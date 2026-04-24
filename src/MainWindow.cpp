#include "MainWindow.h"

#include "ServiceManager.h"
#include "ServiceWebView.h"
#include "TrayIcon.h"

#include <QAction>
#include <QCloseEvent>
#include <QIcon>
#include <QListWidget>
#include <QListWidgetItem>
#include <QSplitter>
#include <QStackedWidget>

#include <KActionCollection>
#include <KLocalizedString>
#include <KStandardAction>

namespace
{
constexpr int kSidebarWidth = 72;
constexpr int kSidebarIconSize = 40;
}

MainWindow::MainWindow(QWidget *parent)
    : KXmlGuiWindow(parent)
    , m_services(new ServiceManager(this))
    , m_tray(new TrayIcon(this))
{
    setupUi();
    setupActions();
    setupGUI(Default, QStringLiteral("syltrui.rc"));

    connect(m_services, &ServiceManager::servicesChanged,
            this, &MainWindow::rebuildServiceViews);
    connect(m_sidebar, &QListWidget::currentRowChanged,
            this, &MainWindow::onServiceSelected);
    connect(m_tray, &TrayIcon::toggleRequested, this, &MainWindow::toggleVisible);

    rebuildServiceViews();
    resize(1200, 780);
}

void MainWindow::setupUi()
{
    m_sidebar = new QListWidget(this);
    m_sidebar->setFixedWidth(kSidebarWidth);
    m_sidebar->setIconSize({kSidebarIconSize, kSidebarIconSize});
    m_sidebar->setSpacing(4);
    m_sidebar->setUniformItemSizes(true);
    m_sidebar->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_sidebar->setFrameShape(QFrame::NoFrame);

    m_stack = new QStackedWidget(this);

    auto *splitter = new QSplitter(Qt::Horizontal, this);
    splitter->addWidget(m_sidebar);
    splitter->addWidget(m_stack);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setCollapsible(0, false);

    setCentralWidget(splitter);
}

void MainWindow::setupActions()
{
    KStandardAction::quit(qApp, &QApplication::quit, actionCollection());
}

void MainWindow::rebuildServiceViews()
{
    m_sidebar->clear();
    while (m_stack->count() > 0) {
        auto *w = m_stack->widget(0);
        m_stack->removeWidget(w);
        w->deleteLater();
    }
    m_views.clear();

    for (const auto &service : m_services->services()) {
        auto *item = new QListWidgetItem(m_sidebar);
        item->setText(service.name());
        item->setToolTip(service.name());
        item->setTextAlignment(Qt::AlignCenter);
        if (!service.iconName().isEmpty()) {
            item->setIcon(QIcon::fromTheme(service.iconName()));
        }

        auto *view = new ServiceWebView(service, m_stack);
        m_stack->addWidget(view);
        m_views.insert(service.id(), view);
    }

    if (m_sidebar->count() > 0) {
        m_sidebar->setCurrentRow(0);
    }
}

void MainWindow::onServiceSelected(int row)
{
    if (row >= 0 && row < m_stack->count()) {
        m_stack->setCurrentIndex(row);
    }
}

void MainWindow::toggleVisible()
{
    if (isVisible() && !isMinimized()) {
        hide();
    } else {
        showNormal();
        raise();
        activateWindow();
    }
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    hide();
    event->ignore();
}
