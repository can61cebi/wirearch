import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import org.kde.kirigami as Kirigami
import org.kde.wirearch

Kirigami.ApplicationWindow {
    id: root

    title: i18nc("@title:window", "WireArch")

    width: Kirigami.Units.gridUnit * 46
    height: Kirigami.Units.gridUnit * 32
    minimumWidth: Kirigami.Units.gridUnit * 26
    minimumHeight: Kirigami.Units.gridUnit * 18

    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.ToolBar
    pageStack.globalToolBar.showNavigationButtons: Kirigami.ApplicationHeaderStyle.ShowBackButton

    // Open a secondary page, replacing any already-open one (avoids duplicates).
    function openPage(component, props) {
        while (root.pageStack.depth > 1) {
            root.pageStack.pop()
        }
        root.pageStack.push(component, props || ({}))
    }

    function fmtBytes(n) {
        if (!n || n < 0) return "0 B"
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let v = n, i = 0
        while (v >= 1024 && i < units.length - 1) { v /= 1024; i++ }
        return (i === 0 ? Math.round(v) : v.toFixed(1)) + " " + units[i]
    }

    function fmtDuration(s) {
        if (!s || s < 0) return "0s"
        s = Math.floor(s)
        const d = Math.floor(s / 86400); s %= 86400
        const h = Math.floor(s / 3600); s %= 3600
        const m = Math.floor(s / 60); const sec = s % 60
        if (d > 0) return d + "d " + h + "h"
        if (h > 0) return h + "h " + m + "m"
        if (m > 0) return m + "m " + sec + "s"
        return sec + "s"
    }

    function fmtHandshake(unix) {
        if (!unix) return i18nc("@info:status", "never")
        const age = Math.max(0, Math.floor(Date.now() / 1000) - unix)
        return i18nc("@info:status, time ago", "%1 ago", fmtDuration(age))
    }

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
                text: i18nc("@action:button", "Add")
                icon.name: "list-add"
                onTriggered: root.openPage(editComponent, { tunnelId: "", tunnelName: "" })
            },
            Kirigami.Action {
                text: i18nc("@action:button", "Import")
                icon.name: "document-import"
                onTriggered: importDialog.open()
            },
            Kirigami.Action {
                text: i18nc("@action:button", "Refresh")
                icon.name: "view-refresh"
                onTriggered: WireArchManager.refresh()
            },
            Kirigami.Action {
                text: i18nc("@action:button", "Statistics")
                icon.name: "office-chart-bar"
                onTriggered: root.openPage(statisticsComponent)
            }
        ]

        ListView {
            id: tunnelList
            model: WireArchManager.tunnels
            spacing: 0

            delegate: Controls.ItemDelegate {
                id: row
                required property var modelData
                readonly property bool isActive: modelData.id === WireArchManager.activeTunnel
                property var geo: WireArchManager.geoFor(modelData.endpoint)
                property var liveStatus: ({})
                property real rxRate: 0
                property real txRate: 0
                property real _prevRx: 0
                property real _prevTx: 0
                property real _prevT: 0

                width: ListView.view.width

                readonly property string subtitleText: {
                    if (row.isActive && row.liveStatus && row.liveStatus.state === "active") {
                        let bits = [i18nc("@info:status", "Connected")]
                        if (row.geo && row.geo.country) bits.push(row.geo.country)
                        bits.push(root.fmtBytes(row.rxRate) + "/s ↓  "
                                  + root.fmtBytes(row.txRate) + "/s ↑")
                        bits.push(root.fmtDuration(row.liveStatus.sinceConnected))
                        return bits.join("   ·   ")
                    }
                    let parts = []
                    if (row.geo && row.geo.country) parts.push(row.geo.country)
                    if (row.geo && row.geo.asOrg) parts.push(row.geo.asOrg)
                    if (parts.length === 0) parts.push(row.modelData.endpoint)
                    return parts.join("   ·   ")
                }

                readonly property string detailText: {
                    let lines = []
                    if (row.geo && row.geo.country)
                        lines.push(i18n("Country: %1", row.geo.country))
                    if (row.geo && row.geo.asOrg)
                        lines.push(i18n("Provider: %1 (AS%2)", row.geo.asOrg, row.geo.asn || 0))
                    if (row.geo && row.geo.ip)
                        lines.push(i18n("Server: %1", row.geo.ip))
                    if (row.isActive && row.liveStatus && row.liveStatus.state === "active") {
                        lines.push(i18n("Received: %1", root.fmtBytes(row.liveStatus.rxBytes)))
                        lines.push(i18n("Sent: %1", root.fmtBytes(row.liveStatus.txBytes)))
                        lines.push(i18n("Last handshake: %1", root.fmtHandshake(row.liveStatus.lastHandshake)))
                        lines.push(i18n("This session: %1", root.fmtDuration(row.liveStatus.sinceConnected)))
                    }
                    if (row.liveStatus && row.liveStatus.totalConnected)
                        lines.push(i18n("Total connected: %1", root.fmtDuration(row.liveStatus.totalConnected)))
                    return lines.join("\n")
                }

                Connections {
                    target: WireArchManager
                    function onGeoUpdated(endpoint) {
                        if (endpoint === row.modelData.endpoint)
                            row.geo = WireArchManager.geoFor(row.modelData.endpoint)
                    }
                    function onActiveTunnelChanged() {
                        if (!row.isActive) {
                            row.liveStatus = ({})
                            row.rxRate = 0
                            row.txRate = 0
                            row._prevT = 0
                        }
                    }
                }

                Timer {
                    running: row.isActive
                    interval: 2000
                    triggeredOnStart: true
                    repeat: true
                    onTriggered: {
                        const s = WireArchManager.statusFor(row.modelData.id)
                        const now = Date.now()
                        if (row._prevT > 0 && s.state === "active") {
                            const dt = (now - row._prevT) / 1000
                            if (dt > 0) {
                                row.rxRate = Math.max(0, ((s.rxBytes || 0) - row._prevRx) / dt)
                                row.txRate = Math.max(0, ((s.txBytes || 0) - row._prevTx) / dt)
                            }
                        }
                        row._prevRx = s.rxBytes || 0
                        row._prevTx = s.txBytes || 0
                        row._prevT = now
                        row.liveStatus = s
                    }
                }

                Controls.ToolTip.text: row.detailText
                Controls.ToolTip.visible: hovered && row.detailText !== ""
                Controls.ToolTip.delay: 700

                contentItem: RowLayout {
                    spacing: Kirigami.Units.largeSpacing

                    Item {
                        Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                        Layout.preferredHeight: Math.round(Kirigami.Units.iconSizes.medium * 0.75)

                        Kirigami.Icon {
                            anchors.fill: parent
                            source: "network-vpn"
                            visible: flagImg.status !== Image.Ready
                        }
                        Image {
                            id: flagImg
                            anchors.fill: parent
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            source: (row.geo && row.geo.countryCode)
                                ? WireArchManager.flagSource(row.geo.countryCode) : ""
                            visible: status === Image.Ready
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Controls.Label {
                            text: row.modelData.name
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        Controls.Label {
                            text: row.subtitleText
                            color: row.isActive ? Kirigami.Theme.positiveTextColor
                                                : Kirigami.Theme.textColor
                            opacity: row.isActive ? 1.0 : 0.7
                            font: Kirigami.Theme.smallFont
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Controls.Button {
                        text: row.isActive ? i18nc("@action:button", "Disconnect")
                                           : i18nc("@action:button", "Connect")
                        icon.name: row.isActive ? "network-disconnect" : "network-connect"
                        onClicked: {
                            if (row.isActive) {
                                WireArchManager.disconnectTunnel(row.modelData.id)
                            } else {
                                WireArchManager.connectTunnel(row.modelData.id)
                            }
                        }
                    }

                    Controls.ToolButton {
                        icon.name: "document-edit"
                        display: Controls.AbstractButton.IconOnly
                        Controls.ToolTip.text: i18nc("@info:tooltip", "Edit tunnel")
                        Controls.ToolTip.visible: hovered
                        onClicked: root.openPage(editComponent,
                            { tunnelId: row.modelData.id, tunnelName: row.modelData.name })
                    }
                    Controls.ToolButton {
                        icon.name: "edit-delete"
                        display: Controls.AbstractButton.IconOnly
                        enabled: !row.isActive
                        Controls.ToolTip.text: i18nc("@info:tooltip", "Remove tunnel")
                        Controls.ToolTip.visible: hovered
                        onClicked: WireArchManager.removeTunnel(row.modelData.id)
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

    Component {
        id: statisticsComponent
        StatisticsPage {}
    }

    Component {
        id: editComponent
        TunnelEditPage {}
    }

    FileDialog {
        id: importDialog
        title: i18nc("@title:window", "Import WireGuard configuration")
        nameFilters: [i18n("WireGuard configurations (*.conf)"), i18n("All files (*)")]
        onAccepted: WireArchManager.importFile("", selectedFile.toString())
    }
}
