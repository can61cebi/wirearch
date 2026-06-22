#include "tray.h"

#include "manager.h"

#include <QAction>
#include <QCoreApplication>
#include <QIcon>
#include <QMenu>
#include <QWindow>

#include <KLocalizedString>
#include <KStatusNotifierItem>

Tray::Tray(WireArchManager *manager, QWindow *window, QObject *parent)
    : QObject(parent)
    , m_manager(manager)
    , m_window(window)
{
    m_sni = new KStatusNotifierItem(QStringLiteral("wirearch"), this);
    m_sni->setTitle(i18n("WireArch"));
    m_sni->setCategory(KStatusNotifierItem::ApplicationStatus);
    m_sni->setStandardActionsEnabled(false);
    m_sni->setIconByName(QStringLiteral("network-vpn"));

    connect(m_sni, &KStatusNotifierItem::activateRequested, this,
            [this](bool, const QPoint &) { showWindow(); });
    connect(m_manager, &WireArchManager::tunnelsChanged, this, &Tray::rebuild);
    connect(m_manager, &WireArchManager::activeTunnelChanged, this, &Tray::rebuild);

    rebuild();
}

void Tray::showWindow()
{
    if (!m_window) {
        return;
    }
    m_window->show();
    m_window->raise();
    m_window->requestActivate();
}

void Tray::rebuild()
{
    const QString active = m_manager->activeTunnel();
    m_sni->setStatus(active.isEmpty() ? KStatusNotifierItem::Passive : KStatusNotifierItem::Active);
    m_sni->setToolTip(QStringLiteral("network-vpn"), i18n("WireArch"),
                      active.isEmpty() ? i18n("Not connected") : i18n("Connected"));

    QMenu *menu = m_sni->contextMenu();
    menu->clear();

    if (!active.isEmpty()) {
        QAction *disconnect = menu->addAction(
            QIcon::fromTheme(QStringLiteral("network-disconnect")), i18n("Disconnect"));
        connect(disconnect, &QAction::triggered, this,
                [this, active] { m_manager->disconnectTunnel(active); });
        menu->addSeparator();
    }

    const QVariantList tunnels = m_manager->tunnels();
    for (const QVariant &entry : tunnels) {
        const QVariantMap tunnel = entry.toMap();
        const QString id = tunnel.value(QStringLiteral("id")).toString();
        const QString name = tunnel.value(QStringLiteral("name")).toString();
        QAction *action = menu->addAction(name);
        action->setCheckable(true);
        action->setChecked(id == active);
        connect(action, &QAction::triggered, this, [this, id] { m_manager->connectTunnel(id); });
    }

    menu->addSeparator();
    QAction *open =
        menu->addAction(QIcon::fromTheme(QStringLiteral("window")), i18n("Open WireArch"));
    connect(open, &QAction::triggered, this, [this] { showWindow(); });
    QAction *quit =
        menu->addAction(QIcon::fromTheme(QStringLiteral("application-exit")), i18n("Quit"));
    connect(quit, &QAction::triggered, qApp, &QCoreApplication::quit);
}
