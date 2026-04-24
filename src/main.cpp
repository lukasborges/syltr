#include "MainWindow.h"

#include <QApplication>
#include <QCommandLineParser>
#include <QIcon>

#include <KAboutData>
#include <KCrash>
#include <KLocalizedString>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    KLocalizedString::setApplicationDomain("syltr");

    KAboutData about(
        QStringLiteral("syltr"),
        i18n("Syltr"),
        QStringLiteral(SYLTR_VERSION_STRING),
        i18n("All-in-one messaging client for KDE Plasma"),
        KAboutLicense::GPL_V3,
        i18n("(c) 2026 Syltr contributors"),
        QString(),
        QStringLiteral("https://github.com/syltr/syltr"));
    about.setOrganizationDomain("syltr.dev");
    about.setDesktopFileName(QStringLiteral("dev.syltr.Syltr"));

    KAboutData::setApplicationData(about);
    KCrash::initialize();

    app.setWindowIcon(QIcon::fromTheme(QStringLiteral("dev.syltr.Syltr"),
                                       QIcon::fromTheme(QStringLiteral("internet-mail"))));
    app.setQuitOnLastWindowClosed(false);

    QCommandLineParser parser;
    about.setupCommandLine(&parser);
    parser.process(app);
    about.processCommandLine(&parser);

    auto *window = new MainWindow();
    window->show();

    return app.exec();
}
