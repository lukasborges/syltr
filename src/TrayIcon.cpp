#include "TrayIcon.h"

#include <QAction>
#include <QCoreApplication>
#include <QIcon>
#include <QMenu>

#include <KLocalizedString>
#include <KStatusNotifierItem>

TrayIcon::TrayIcon(QObject *parent)
    : QObject(parent)
    , m_item(new KStatusNotifierItem(QStringLiteral("syltr"), this))
{
    m_item->setTitle(QStringLiteral("Syltr"));
    m_item->setToolTipTitle(QStringLiteral("Syltr"));
    m_item->setToolTipSubTitle(i18n("All-in-one messaging"));
    m_item->setIconByName(QStringLiteral("dev.syltr.Syltr"));
    m_item->setToolTipIconByName(QStringLiteral("dev.syltr.Syltr"));
    m_item->setCategory(KStatusNotifierItem::Communications);
    m_item->setStatus(KStatusNotifierItem::Active);
    m_item->setStandardActionsEnabled(true);

    auto *menu = new QMenu();
    auto *toggle = menu->addAction(QIcon::fromTheme(QStringLiteral("view-visible")),
                                   i18n("Show / Hide"));
    connect(toggle, &QAction::triggered, this, &TrayIcon::toggleRequested);
    menu->addSeparator();
    auto *quit = menu->addAction(QIcon::fromTheme(QStringLiteral("application-exit")),
                                 i18n("Quit"));
    connect(quit, &QAction::triggered, QCoreApplication::instance(), &QCoreApplication::quit);
    m_item->setContextMenu(menu);

    connect(m_item, &KStatusNotifierItem::activateRequested,
            this, [this]() { Q_EMIT toggleRequested(); });
}
