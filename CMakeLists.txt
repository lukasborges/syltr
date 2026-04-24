cmake_minimum_required(VERSION 3.22)

project(Syltr
    VERSION 0.1.0
    DESCRIPTION "All-in-one messaging client for KDE Plasma"
    HOMEPAGE_URL "https://github.com/syltr/syltr"
    LANGUAGES CXX
)

set(QT_MIN_VERSION 6.5.0)
set(KF_MIN_VERSION 6.0.0)

find_package(ECM ${KF_MIN_VERSION} REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)
include(ECMInstallIcons)
include(FeatureSummary)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Qt6 ${QT_MIN_VERSION} REQUIRED COMPONENTS
    Core
    Widgets
    WebEngineWidgets
    Network
    Svg
)

find_package(KF6 ${KF_MIN_VERSION} REQUIRED COMPONENTS
    CoreAddons
    I18n
    XmlGui
    ConfigWidgets
    Notifications
    StatusNotifierItem
    WidgetsAddons
    Crash
    WindowSystem
)

add_subdirectory(src)
add_subdirectory(data)

feature_summary(WHAT ALL INCLUDE_QUIET_PACKAGES FATAL_ON_MISSING_REQUIRED_PACKAGES)
