import QtQuick
import QtQuick.Controls
import QtWebEngine

Item {
    id: root
    property string serviceId
    property string serviceUrl
    property var profile

    WebEngineView {
        id: view
        anchors.fill: parent
        profile: root.profile
        url: root.serviceUrl

        onFeaturePermissionRequested: (securityOrigin, feature) => {
            view.grantFeaturePermission(securityOrigin, feature, true);
        }
    }

    BusyIndicator {
        anchors.centerIn: parent
        running: view.loading
        visible: view.loading
    }
}
