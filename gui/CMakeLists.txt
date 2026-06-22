add_executable(wirearch)

target_include_directories(wirearch PRIVATE src)

target_sources(wirearch PRIVATE
    src/main.cpp
)

qt_add_qml_module(wirearch
    URI org.kde.wirearch
    QML_FILES
        qml/Main.qml
    SOURCES
        src/manager.h
        src/manager.cpp
)

target_link_libraries(wirearch PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Widgets
    Qt6::DBus
    KF6::CoreAddons
    KF6::I18n
    KF6::I18nQml
)

install(TARGETS wirearch ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
