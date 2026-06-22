import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import org.kde.wirearch

Kirigami.ApplicationWindow {
    id: root

    title: i18nc("@title:window", "WireArch")

    width: Kirigami.Units.gridUnit * 44
    height: Kirigami.Units.gridUnit * 32
    minimumWidth: Kirigami.Units.gridUnit * 24
    minimumHeight: Kirigami.Units.gridUnit * 18

    Connections {
        target: WireArchManager
        function onErrorOccurred(message) {
            root.showPassiveNotification(message, "long")
        }
    }

    pageStack.initialPage: Kirigami.ScrollablePage {
        id: tunnelsPage
        title: i18nc("@title", "Tunnels")

        actions: [
            Kirigami.Action {
                text: i18nc("@action:button", "Import")
                icon.name: "document-import"
                onTriggered: importDialog.open()
            },
            Kirigami.Action {
                text: i18nc("@action:button", "Refresh")
                icon.name: "view-refresh"
                onTriggered: WireArchManager.refresh()
            }
        ]

        ListView {
            id: tunnelList
            model: WireArchManager.tunnels
            spacing: 0

            delegate: Controls.ItemDelegate {
                id: delegateItem
                required property var modelData
                width: ListView.view.width

                contentItem: RowLayout {
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.Icon {
                        source: "network-vpn"
                        Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                        Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Controls.Label {
                            text: delegateItem.modelData.name
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        Controls.Label {
                            text: delegateItem.modelData.endpoint
                            font: Kirigami.Theme.smallFont
                            opacity: 0.7
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Controls.ToolButton {
                        icon.name: "edit-delete"
                        display: Controls.AbstractButton.IconOnly
                        Controls.ToolTip.text: i18nc("@info:tooltip", "Remove tunnel")
                        Controls.ToolTip.visible: hovered
                        onClicked: WireArchManager.removeTunnel(delegateItem.modelData.id)
                    }
                }
            }

            Kirigami.PlaceholderMessage {
                anchors.centerIn: parent
                width: parent.width - Kirigami.Units.gridUnit * 4
                visible: tunnelList.count === 0
                icon.name: WireArchManager.available ? "network-vpn" : "network-disconnect"
                text: WireArchManager.available
                    ? i18nc("@info:placeholder", "No tunnels yet")
                    : i18nc("@info:placeholder", "Service unavailable")
                explanation: WireArchManager.available
                    ? i18n("Import a WireGuard configuration to add your first tunnel.")
                    : i18n("The WireArch background service is not running.")

                helpfulAction: Kirigami.Action {
                    enabled: WireArchManager.available
                    text: i18nc("@action:button", "Import configuration")
                    icon.name: "document-import"
                    onTriggered: importDialog.open()
                }
            }
        }
    }

    FileDialog {
        id: importDialog
        title: i18nc("@title:window", "Import WireGuard configuration")
        nameFilters: [i18n("WireGuard configurations (*.conf)"), i18n("All files (*)")]
        onAccepted: WireArchManager.importFile("", selectedFile.toString())
    }
}
