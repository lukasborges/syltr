import QtQuick
import QtQuick.Controls
import QtWebEngine

Item {
    id: root
    property string serviceId
    property string serviceUrl

    WebEngineProfile {
        id: serviceProfile
        storageName: "syltr-" + root.serviceId
        offTheRecord: false
        persistentStoragePath: paths.profilePath(root.serviceId)
        cachePath: paths.cachePath(root.serviceId)
        httpUserAgent: userAgent
        persistentCookiesPolicy: WebEngineProfile.ForcePersistentCookies
    }

    WebEngineView {
        id: view
        anchors.fill: parent
        profile: serviceProfile
        url: root.serviceUrl
        settings.playbackRequiresUserGesture: false
        settings.showScrollBars: false

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
