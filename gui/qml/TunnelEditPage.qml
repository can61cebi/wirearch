import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kirigamiaddons.formcard as FormCard
import org.kde.wirearch

Kirigami.ScrollablePage {
    id: page

    property string tunnelId: ""
    property string tunnelName: ""

    title: page.tunnelId === "" ? i18nc("@title", "New tunnel") : i18nc("@title", "Edit tunnel")

    function setPrivateKey(key) {
        let text = configArea.text
        if (/PrivateKey\s*=/.test(text)) {
            text = text.replace(/PrivateKey\s*=.*/, "PrivateKey = " + key)
        } else if (text.indexOf("[Interface]") !== -1) {
            text = text.replace("[Interface]", "[Interface]\nPrivateKey = " + key)
        } else {
            text = "[Interface]\nPrivateKey = " + key + "\n" + text
        }
        configArea.text = text
    }

    Component.onCompleted: {
        nameField.text = page.tunnelName
        if (page.tunnelId !== "") {
            configArea.text = WireArchManager.getConfig(page.tunnelId)
        } else {
            configArea.text = "[Interface]\nPrivateKey = \nAddress = \nDNS = \n\n"
                + "[Peer]\nPublicKey = \nEndpoint = \nAllowedIPs = 0.0.0.0/0, ::/0\nPersistentKeepalive = 25\n"
        }
    }

    ColumnLayout {
        spacing: 0

        FormCard.FormHeader { title: i18nc("@title:group", "Tunnel") }
        FormCard.FormCard {
            FormCard.FormTextFieldDelegate {
                id: nameField
                label: i18nc("@label:textbox", "Name")
                placeholderText: i18nc("@info:placeholder", "e.g. Hetzner DE")
            }
        }

        FormCard.FormHeader { title: i18nc("@title:group", "Configuration") }
        FormCard.FormCard {
            FormCard.FormTextAreaDelegate {
                id: configArea
                label: i18nc("@label:textbox", "WireGuard configuration")
            }
            FormCard.FormDelegateSeparator {}
            FormCard.FormButtonDelegate {
                text: i18nc("@action:button", "Generate keypair")
                icon.name: "lock"
                onClicked: {
                    const kp = WireArchManager.generateKeypair()
                    if (kp.privateKey) {
                        page.setPrivateKey(kp.privateKey)
                        applicationWindow().showPassiveNotification(
                            i18n("Public key (give this to the server): %1", kp.publicKey), "long")
                    }
                }
            }
        }

        FormCard.FormCard {
            Layout.topMargin: Kirigami.Units.largeSpacing
            FormCard.FormButtonDelegate {
                text: i18nc("@action:button", "Save")
                icon.name: "document-save"
                onClicked: {
                    const id = WireArchManager.saveTunnel(page.tunnelId, nameField.text, configArea.text)
                    if (id !== "") {
                        applicationWindow().pageStack.pop()
                    }
                }
            }
        }
    }
}
