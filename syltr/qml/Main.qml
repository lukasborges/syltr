import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtWebEngine

ApplicationWindow {
    id: root
    width: 1200
    height: 780
    minimumWidth: 720
    minimumHeight: 480
    visible: true
    title: qsTr("Syltr")

    Connections {
        target: appBridge
        function onToggleWindow() {
            if (root.visible) {
                root.hide();
            } else {
                root.show();
                root.raise();
                root.requestActivate();
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Sidebar {
            id: sidebar
            Layout.fillHeight: true
            Layout.preferredWidth: 72
            model: serviceModel
            onServiceSelected: (row) => stack.currentIndex = row
        }

        StackLayout {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: 0

            Repeater {
                model: serviceModel
                delegate: ServiceView {
                    serviceId: model.serviceId
                    serviceUrl: model.url
                }
            }
        }
    }

    Label {
        anchors.centerIn: parent
        visible: serviceModel.rowCount() === 0
        text: qsTr("No services configured.\nEdit services.json to add one.")
        horizontalAlignment: Text.AlignHCenter
        opacity: 0.6
    }
}
