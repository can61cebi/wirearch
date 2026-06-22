import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.wirearch

Kirigami.ScrollablePage {
    id: page
    title: i18nc("@title", "Statistics")

    property string period: "hour"
    property var buckets: []
    property real maxVal: 1
    property double totalRx: 0
    property double totalTx: 0

    function fmtBytes(n) {
        if (!n || n < 0) return "0 B"
        const u = ["B", "KiB", "MiB", "GiB", "TiB"]
        let v = n, i = 0
        while (v >= 1024 && i < u.length - 1) { v /= 1024; i++ }
        return (i === 0 ? Math.round(v) : v.toFixed(1)) + " " + u[i]
    }

    function reload() {
        const data = WireArchManager.metrics(period, period === "hour" ? 24 : 30)
        let mx = 1, trx = 0, ttx = 0
        for (let i = 0; i < data.length; i++) {
            const rx = data[i].rx || 0
            const tx = data[i].tx || 0
            if (rx + tx > mx) mx = rx + tx
            trx += rx
            ttx += tx
        }
        buckets = data
        maxVal = mx
        totalRx = trx
        totalTx = ttx
    }

    Component.onCompleted: reload()

    actions: [
        Kirigami.Action {
            text: i18nc("@action", "Hourly")
            checkable: true
            checked: page.period === "hour"
            onTriggered: { page.period = "hour"; page.reload() }
        },
        Kirigami.Action {
            text: i18nc("@action", "Daily")
            checkable: true
            checked: page.period === "day"
            onTriggered: { page.period = "day"; page.reload() }
        },
        Kirigami.Action {
            text: i18nc("@action:button", "Refresh")
            icon.name: "view-refresh"
            onTriggered: page.reload()
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.AbstractCard {
            Layout.fillWidth: true
            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing * 2
                ColumnLayout {
                    spacing: 0
                    Controls.Label {
                        text: i18nc("@label", "Downloaded")
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                    }
                    Controls.Label { text: page.fmtBytes(page.totalRx); font.bold: true }
                }
                ColumnLayout {
                    spacing: 0
                    Controls.Label {
                        text: i18nc("@label", "Uploaded")
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                    }
                    Controls.Label { text: page.fmtBytes(page.totalTx); font.bold: true }
                }
                Item { Layout.fillWidth: true }
                Controls.Label {
                    text: page.period === "hour" ? i18n("Last 24 hours") : i18n("Last 30 days")
                    opacity: 0.7
                }
            }
        }

        Item {
            id: chart
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.gridUnit * 12

            Controls.Label {
                anchors.centerIn: parent
                visible: page.buckets.length === 0
                text: i18n("No usage recorded yet")
                opacity: 0.6
            }

            Row {
                anchors.fill: parent
                spacing: 2
                visible: page.buckets.length > 0
                Repeater {
                    model: page.buckets
                    delegate: Item {
                        required property var modelData
                        height: chart.height
                        width: (chart.width - (page.buckets.length - 1) * 2)
                               / Math.max(1, page.buckets.length)

                        Rectangle {
                            id: rxBar
                            anchors.bottom: parent.bottom
                            width: parent.width
                            radius: 2
                            height: chart.height * ((modelData.rx || 0) / page.maxVal)
                            color: Kirigami.Theme.highlightColor
                        }
                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: rxBar.height
                            width: parent.width
                            radius: 2
                            height: chart.height * ((modelData.tx || 0) / page.maxVal)
                            color: Kirigami.Theme.neutralTextColor
                            opacity: 0.7
                        }

                        HoverHandler { id: barHover }
                        Controls.ToolTip.visible: barHover.hovered
                        Controls.ToolTip.text: page.fmtBytes((modelData.rx || 0) + (modelData.tx || 0))
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            Rectangle { width: 12; height: 12; radius: 2; color: Kirigami.Theme.highlightColor }
            Controls.Label { text: i18nc("@label", "Download"); font: Kirigami.Theme.smallFont }
            Item { width: Kirigami.Units.largeSpacing }
            Rectangle {
                width: 12; height: 12; radius: 2
                color: Kirigami.Theme.neutralTextColor; opacity: 0.7
            }
            Controls.Label { text: i18nc("@label", "Upload"); font: Kirigami.Theme.smallFont }
            Item { Layout.fillWidth: true }
        }
    }
}
