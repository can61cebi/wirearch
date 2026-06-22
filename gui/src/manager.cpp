#include "manager.h"

#include <QCoreApplication>
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QDBusPendingCallWatcher>
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

QString WireArchManager::activeTunnel() const
{
    return m_activeTunnel;
}

void WireArchManager::setAvailable(bool available)
{
    if (m_available != available) {
        m_available = available;
        Q_EMIT availableChanged();
    }
}

void WireArchManager::refreshActive()
{
    if (!m_iface) {
        return;
    }
    const QString id = m_iface->property("ActiveTunnel").toString();
    if (id != m_activeTunnel) {
        m_activeTunnel = id;
        Q_EMIT activeTunnelChanged();
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
    const auto list = qdbus_cast<QList<QVariantMap>>(reply.arguments().value(0));
    for (const QVariantMap &tunnel : list) {
        tunnels.append(tunnel);
    }

    m_tunnels = tunnels;
    setAvailable(true);
    Q_EMIT tunnelsChanged();
    refreshActive();
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

QString WireArchManager::busyTunnel() const
{
    return m_busyTunnel;
}

// Call a privileged method asynchronously so the UI never blocks (Connect may
// take several seconds while the daemon verifies the handshake).
void WireArchManager::callAsync(const QString &method, const QString &id)
{
    if (!m_iface) {
        return;
    }
    m_busyTunnel = id;
    Q_EMIT busyTunnelChanged();
    auto *watcher = new QDBusPendingCallWatcher(m_iface->asyncCall(method, id), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *w) {
                const QDBusMessage reply = w->reply();
                if (reply.type() == QDBusMessage::ErrorMessage) {
                    Q_EMIT errorOccurred(reply.errorMessage());
                }
                m_busyTunnel.clear();
                Q_EMIT busyTunnelChanged();
                refreshActive();
                w->deleteLater();
            });
}

void WireArchManager::connectTunnel(const QString &id)
{
    callAsync(QStringLiteral("Connect"), id);
}

void WireArchManager::disconnectTunnel(const QString &id)
{
    callAsync(QStringLiteral("Disconnect"), id);
}

QVariantMap WireArchManager::geoFor(const QString &endpoint)
{
    if (endpoint.isEmpty() || !m_iface) {
        return QVariantMap();
    }
    if (m_geoCache.contains(endpoint)) {
        return m_geoCache.value(endpoint);
    }
    // Mark in-flight (empty) to avoid duplicate calls, then fetch async.
    m_geoCache.insert(endpoint, QVariantMap());
    const QDBusPendingCall pending = m_iface->asyncCall(QStringLiteral("Geo"), endpoint);
    auto *watcher = new QDBusPendingCallWatcher(pending, this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, endpoint](QDBusPendingCallWatcher *w) {
                const QDBusMessage reply = w->reply();
                if (reply.type() != QDBusMessage::ErrorMessage) {
                    m_geoCache.insert(endpoint,
                                      qdbus_cast<QVariantMap>(reply.arguments().value(0)));
                    Q_EMIT geoUpdated(endpoint);
                }
                w->deleteLater();
            });
    return QVariantMap();
}

QVariantMap WireArchManager::statusFor(const QString &id)
{
    if (!m_iface) {
        return QVariantMap();
    }
    const QDBusMessage reply = m_iface->call(QStringLiteral("GetStatus"), id);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        return QVariantMap();
    }
    return qdbus_cast<QVariantMap>(reply.arguments().value(0));
}

QVariantList WireArchManager::metrics(const QString &period, int count)
{
    if (!m_iface) {
        return QVariantList();
    }
    const QDBusMessage reply =
        m_iface->call(QStringLiteral("GetMetrics"), period, static_cast<uint>(count));
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return QVariantList();
    }
    QVariantList out;
    const auto list = qdbus_cast<QList<QVariantMap>>(reply.arguments().value(0));
    for (const QVariantMap &bucket : list) {
        out.append(bucket);
    }
    return out;
}

QString WireArchManager::flagSource(const QString &countryCode) const
{
    if (countryCode.isEmpty()) {
        return QString();
    }
    return QStringLiteral("qrc:/flags/%1.svg").arg(countryCode.toLower());
}

QString WireArchManager::saveTunnel(const QString &id, const QString &name, const QString &config)
{
    if (!m_iface) {
        return QString();
    }
    const QDBusMessage reply =
        m_iface->call(QStringLiteral("SaveTunnel"), id, name, config);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return QString();
    }
    refresh();
    return reply.arguments().value(0).toString();
}

QString WireArchManager::getConfig(const QString &id)
{
    if (!m_iface) {
        return QString();
    }
    const QDBusMessage reply = m_iface->call(QStringLiteral("GetTunnel"), id);
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return QString();
    }
    const QVariantMap tunnel = qdbus_cast<QVariantMap>(reply.arguments().value(0));
    return tunnel.value(QStringLiteral("config")).toString();
}

QVariantMap WireArchManager::generateKeypair()
{
    if (!m_iface) {
        return QVariantMap();
    }
    const QDBusMessage reply = m_iface->call(QStringLiteral("GenerateKeypair"));
    if (reply.type() == QDBusMessage::ErrorMessage) {
        Q_EMIT errorOccurred(reply.errorMessage());
        return QVariantMap();
    }
    QVariantMap result;
    result[QStringLiteral("privateKey")] = reply.arguments().value(0).toString();
    result[QStringLiteral("publicKey")] = reply.arguments().value(1).toString();
    return result;
}
