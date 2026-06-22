#include "manager.h"

#include <QCoreApplication>
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QFile>
#include <QFileInfo>
#include <QUrl>
#include <QVariantMap>

namespace
{
const QString Service = QStringLiteral("tr.cebi.wirearch");
const QString Path = QStringLiteral("/tr/cebi/wirearch");
const QString Iface = QStringLiteral("tr.cebi.wirearch.Manager");
}

WireArchManager::WireArchManager(QObject *parent)
    : QObject(parent)
{
    const bool useSession = qEnvironmentVariableIsSet("WIREARCH_SESSION_BUS")
        || QCoreApplication::arguments().contains(QStringLiteral("--session"));
    const QDBusConnection bus =
        useSession ? QDBusConnection::sessionBus() : QDBusConnection::systemBus();
    m_iface = new QDBusInterface(Service, Path, Iface, bus, this);
    refresh();
}

QVariantList WireArchManager::tunnels() const
{
    return m_tunnels;
}

bool WireArchManager::available() const
{
    return m_available;
}

void WireArchManager::setAvailable(bool available)
{
    if (m_available != available) {
        m_available = available;
        Q_EMIT availableChanged();
    }
}

void WireArchManager::refresh()
{
    if (!m_iface) {
        return;
    }
    const QDBusMessage reply = m_iface->call(QStringLiteral("ListTunnels"));
    if (reply.type() == QDBusMessage::ErrorMessage) {
        setAvailable(false);
        Q_EMIT errorOccurred(reply.errorMessage());
        return;
    }

    QVariantList tunnels;
    QDBusArgument arg = reply.arguments().value(0).value<QDBusArgument>();
    arg.beginArray();
    while (!arg.atEnd()) {
        QVariantMap map;
        arg >> map;
        tunnels.append(map);
    }
    arg.endArray();

    m_tunnels = tunnels;
    setAvailable(true);
    Q_EMIT tunnelsChanged();
}

QString WireArchManager::importFile(const QString &name, const QString &fileUrl)
{
    QString path = fileUrl;
    if (path.startsWith(QStringLiteral("file://"))) {
        path = QUrl(fileUrl).toLocalFile();
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        Q_EMIT errorOccurred(QStringLiteral("Cannot open %1").arg(path));
        return QString();
    }
    const QString text = QString::fromUtf8(file.readAll());
    QString displayName = name;
    if (displayName.isEmpty()) {
        displayName = QFileInfo(path).completeBaseName();
    }
    return importText(displayName, text);
}

QString WireArchManager::importText(const QString &name, const QString &configText)
{
    if (!m_iface) {
        return QString();
    }
    const QDBusMessage reply =
        m_iface->call(QStringLiteral("ImportConfig"), name, configText);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return QString();
    }
    refresh();
    return reply.arguments().value(0).toString();
}

void WireArchManager::removeTunnel(const QString &id)
{
    if (!m_iface) {
        return;
    }
    const QDBusMessage reply = m_iface->call(QStringLiteral("RemoveTunnel"), id);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return;
    }
    refresh();
}
