set(syltr_SRCS
    main.cpp
    MainWindow.cpp
    MainWindow.h
    Service.cpp
    Service.h
    ServiceManager.cpp
    ServiceManager.h
    ServiceWebView.cpp
    ServiceWebView.h
    TrayIcon.cpp
    TrayIcon.h
)

qt_add_resources(syltr_SRCS ../resources/resources.qrc)

add_executable(syltr ${syltr_SRCS})

target_compile_definitions(syltr PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_STRICT_ITERATORS
    SYLTR_VERSION_STRING="${PROJECT_VERSION}"
)

target_link_libraries(syltr PRIVATE
    Qt6::Core
    Qt6::Widgets
    Qt6::WebEngineWidgets
    Qt6::Network
    Qt6::Svg
    KF6::CoreAddons
    KF6::I18n
    KF6::XmlGui
    KF6::ConfigWidgets
    KF6::Notifications
    KF6::StatusNotifierItem
    KF6::WidgetsAddons
    KF6::Crash
    KF6::WindowSystem
)

install(TARGETS syltr ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
install(FILES syltrui.rc DESTINATION ${KDE_INSTALL_KXMLGUIDIR}/syltr)
