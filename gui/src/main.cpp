#include <QApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QWindow>

#include <KAboutData>
#include <KDBusService>
#include <KLocalizedContext>
#include <KLocalizedString>

#include "manager.h"
#include "tray.h"

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QApplication::setQuitOnLastWindowClosed(false);

    KLocalizedString::setApplicationDomain(QByteArrayLiteral("wirearch"));
    QApplication::setOrganizationName(QStringLiteral("WireArch"));
    QApplication::setApplicationName(QStringLiteral("WireArch"));
    QApplication::setDesktopFileName(QStringLiteral("org.kde.wirearch"));

    KAboutData aboutData(QStringLiteral("wirearch"),
                         i18n("WireArch"),
                         QStringLiteral("0.1.0"),
                         i18n("A native KDE WireGuard VPN client"),
                         KAboutLicense::GPL_V3,
                         i18n("(c) 2026 WireArch"));
    aboutData.setHomepage(QStringLiteral("https://github.com/can61cebi/wirearch"));
    KAboutData::setApplicationData(aboutData);

    // Single instance: a second launch just raises the running window.
    KDBusService service(KDBusService::Unique);

    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextObject(new KLocalizedContext(&engine));
    engine.loadFromModule("org.kde.wirearch", "Main");
    if (engine.rootObjects().isEmpty()) {
        return -1;
    }

    auto *manager =
        engine.singletonInstance<WireArchManager *>("org.kde.wirearch", "WireArchManager");
    auto *window = qobject_cast<QWindow *>(engine.rootObjects().constFirst());
    if (manager && window) {
        new Tray(manager, window, &app);
    }

    if (window) {
        QObject::connect(&service, &KDBusService::activateRequested, window,
                         [window](const QStringList &, const QString &) {
                             window->show();
                             window->raise();
                             window->requestActivate();
                         });
    }

    return app.exec();
}
