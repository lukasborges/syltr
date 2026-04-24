import QtQuick
import QtQuick.Controls

Rectangle {
    id: root
    color: palette.window

    property alias model: list.model
    signal serviceSelected(int row)

    ListView {
        id: list
        anchors.fill: parent
        spacing: 4
        topMargin: 8
        bottomMargin: 8
        currentIndex: 0
        clip: true

        delegate: ItemDelegate {
            width: root.width
            height: 56
            highlighted: ListView.isCurrentItem
            onClicked: {
                list.currentIndex = index;
                root.serviceSelected(index);
            }

            contentItem: Item {
                Rectangle {
                    anchors.centerIn: parent
                    width: 40
                    height: 40
                    radius: 8
                    color: highlighted ? palette.highlight : palette.button
                    Text {
                        anchors.centerIn: parent
                        text: (model.name || "?").substring(0, 1).toUpperCase()
                        color: highlighted ? palette.highlightedText : palette.buttonText
                        font.bold: true
                        font.pixelSize: 18
                    }
                }
            }

            ToolTip.text: model.name || ""
            ToolTip.visible: hovered
            ToolTip.delay: 400
        }
    }
}
