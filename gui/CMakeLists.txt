add_executable(wirearch)

target_sources(wirearch PRIVATE
    src/main.cpp
)

qt_add_qml_module(wirearch
    URI org.kde.wirearch
    QML_FILES
        qml/Main.qml
)

target_link_libraries(wirearch PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::Widgets
    KF6::CoreAddons
    KF6::I18n
    KF6::I18nQml
)

install(TARGETS wirearch ${KDE_INSTALL_TARGETS_DEFAULT_ARGS})
