import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.plasma5support as P5Support
import org.kde.kirigami as Kirigami

PlasmoidItem {
    id: root

    property string activeTunnel: ""
    readonly property bool connected: activeTunnel.length > 0

    // The daemon lives on the system bus; we talk to it through busctl.
    readonly property string svc: "tr.cebi.wirearch /tr/cebi/wirearch tr.cebi.wirearch.Manager"

    P5Support.DataSource {
        id: exec
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            disconnectSource(source)
            if (source.indexOf("ActiveTunnel") !== -1) {
                const out = ((data["stdout"] || "")).trim()
                const m = out.match(/"([^"]*)"/)
                root.activeTunnel = m ? m[1] : ""
            }
        }
        function run(cmd) { connectSource(cmd) }
    }

    function poll() {
        exec.run("busctl get-property " + root.svc + " ActiveTunnel")
    }

    Timer {
        interval: 3000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.poll()
    }

    Plasmoid.icon: "wirearch-symbolic"
    toolTipSubText: root.connected
        ? i18n("Connected: %1", root.activeTunnel)
        : i18n("Not connected")

    compactRepresentation: Item {
        Kirigami.Icon {
            anchors.fill: parent
            source: "wirearch-symbolic"
            opacity: root.connected ? 1.0 : 0.5
        }
        MouseArea {
            anchors.fill: parent
            onClicked: root.expanded = !root.expanded
        }
    }

    fullRepresentation: ColumnLayout {
        Layout.minimumWidth: Kirigami.Units.gridUnit * 15
        Layout.minimumHeight: Kirigami.Units.gridUnit * 9
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing
            Kirigami.Icon {
                source: "wirearch"
                Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                opacity: root.connected ? 1.0 : 0.5
            }
            ColumnLayout {
                spacing: 0
                PlasmaComponents.Label {
                    text: i18n("WireArch")
                    font.bold: true
                }
                PlasmaComponents.Label {
                    text: root.connected
                        ? i18n("Connected: %1", root.activeTunnel)
                        : i18n("Not connected")
                    opacity: 0.7
                    font: Kirigami.Theme.smallFont
                }
            }
            Item { Layout.fillWidth: true }
        }

        PlasmaComponents.Button {
            visible: root.connected
            text: i18n("Disconnect")
            icon.name: "network-disconnect"
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            onClicked: {
                exec.run("busctl call " + root.svc + " Disconnect s \"" + root.activeTunnel + "\"")
                root.poll()
            }
        }
        PlasmaComponents.Button {
            text: i18n("Open WireArch")
            icon.name: "configure"
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            onClicked: exec.run("wirearch")
        }
        Item { Layout.fillHeight: true }
    }
}
