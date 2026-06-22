import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root

    title: i18nc("@title:window", "WireArch")

    width: Kirigami.Units.gridUnit * 44
    height: Kirigami.Units.gridUnit * 32
    minimumWidth: Kirigami.Units.gridUnit * 24
    minimumHeight: Kirigami.Units.gridUnit * 18

    pageStack.initialPage: Kirigami.ScrollablePage {
        title: i18nc("@title", "Tunnels")

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            icon.name: "network-vpn"
            text: i18n("No tunnels yet")
            explanation: i18n("Import a WireGuard configuration to add your first tunnel.")
        }
    }
}
