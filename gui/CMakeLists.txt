add_executable(wirearch)

target_include_directories(wirearch PRIVATE src)

target_sources(wirearch PRIVATE
    src/main.cpp
)

qt_add_qml_module(wirearch
    URI org.kde.wirearch
    QML_FILES
        qml/Main.qml
        qml/StatisticsPage.qml
        qml/TunnelEditPage.qml
    SOURCES
        src/manager.h
        src/manager.cpp
)

# Bundle the flag-icons (MIT) SVG set as a Qt resource at qrc:/flags/<cc>.svg
file(GLOB WIREARCH_FLAGS CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/flags/*.svg")
qt_add_resources(wirearch "flags"
    PREFIX "/flags"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}/flags"
    FILES ${WIREARCH_FLAGS}
)

target_link_libraries(wirearch PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Widgets
    Qt6::DBus
    Qt6::Svg
    KF6::CoreAddons
    KF6::I18n
    KF6::I18nQml
)

install(TARGETS wirearch ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
