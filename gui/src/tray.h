#pragma once

#include <QObject>

class QWindow;
class KStatusNotifierItem;
class WireArchManager;

/// System tray presence (StatusNotifierItem) with a quick connect/switch menu.
class Tray : public QObject
{
    Q_OBJECT

public:
    Tray(WireArchManager *manager, QWindow *window, QObject *parent = nullptr);

private:
    void rebuild();
    void showWindow();

    KStatusNotifierItem *m_sni = nullptr;
    WireArchManager *m_manager = nullptr;
    QWindow *m_window = nullptr;
};
