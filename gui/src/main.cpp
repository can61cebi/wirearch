#include <QApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>

#include <KAboutData>
#include <KLocalizedContext>
#include <KLocalizedString>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);

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

    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextObject(new KLocalizedContext(&engine));
    engine.loadFromModule("org.kde.wirearch", "Main");
    if (engine.rootObjects().isEmpty()) {
        return -1;
    }

    return app.exec();
}
