#pragma once

#include <QHash>
#include <QObject>
#include <QString>
#include <QVariantList>
#include <QVariantMap>
#include <qqmlregistration.h>

class QDBusInterface;

/// QML-facing client for the tr.cebi.wirearch.Manager D-Bus service.
/// Exposed to QML as a singleton; talks to the daemon over the system bus
/// (or the session bus in dev mode via WIREARCH_SESSION_BUS / --session).
class WireArchManager : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON
    Q_PROPERTY(QVariantList tunnels READ tunnels NOTIFY tunnelsChanged)
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    Q_PROPERTY(QString activeTunnel READ activeTunnel NOTIFY activeTunnelChanged)

public:
    explicit WireArchManager(QObject *parent = nullptr);

    QVariantList tunnels() const;
    bool available() const;
    QString activeTunnel() const;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QString importFile(const QString &name, const QString &fileUrl);
    Q_INVOKABLE QString importText(const QString &name, const QString &configText);
    Q_INVOKABLE void removeTunnel(const QString &id);
    Q_INVOKABLE void connectTunnel(const QString &id);
    Q_INVOKABLE void disconnectTunnel(const QString &id);

    /// Country/ISP for an endpoint. Returns a cached map (empty on first call)
    /// and fetches asynchronously; emits geoUpdated(endpoint) when ready.
    Q_INVOKABLE QVariantMap geoFor(const QString &endpoint);
    /// Live status for a tunnel (synchronous; call periodically while active).
    Q_INVOKABLE QVariantMap statusFor(const QString &id);
    /// Usage rollups for charts ("hour" or "day"), most-recent `count` buckets.
    Q_INVOKABLE QVariantList metrics(const QString &period, int count);
    /// Resource path of the flag for a 2-letter country code (empty if none).
    Q_INVOKABLE QString flagSource(const QString &countryCode) const;

Q_SIGNALS:
    void tunnelsChanged();
    void availableChanged();
    void activeTunnelChanged();
    void geoUpdated(const QString &endpoint);
    void errorOccurred(const QString &message);

private:
    void setAvailable(bool available);
    void refreshActive();

    QDBusInterface *m_iface = nullptr;
    QVariantList m_tunnels;
    bool m_available = false;
    QString m_activeTunnel;
    QHash<QString, QVariantMap> m_geoCache;
};
