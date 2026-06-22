#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
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

public:
    explicit WireArchManager(QObject *parent = nullptr);

    QVariantList tunnels() const;
    bool available() const;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QString importFile(const QString &name, const QString &fileUrl);
    Q_INVOKABLE QString importText(const QString &name, const QString &configText);
    Q_INVOKABLE void removeTunnel(const QString &id);

Q_SIGNALS:
    void tunnelsChanged();
    void availableChanged();
    void errorOccurred(const QString &message);

private:
    void setAvailable(bool available);

    QDBusInterface *m_iface = nullptr;
    QVariantList m_tunnels;
    bool m_available = false;
};
