cmake_minimum_required(VERSION 3.16)
project(wirearch VERSION 0.1.0 LANGUAGES CXX)

set(QT_MIN_VERSION 6.8.0)
set(KF6_MIN_VERSION 6.0.0)

find_package(ECM ${KF6_MIN_VERSION} REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)
include(FeatureSummary)

find_package(Qt6 ${QT_MIN_VERSION} REQUIRED COMPONENTS
    Core
    Gui
    Qml
    Quick
    QuickControls2
    Widgets
    DBus
    Svg
)
qt_standard_project_setup(REQUIRES 6.8)

find_package(KF6 ${KF6_MIN_VERSION} REQUIRED COMPONENTS
    CoreAddons
    I18n
    Config
    Kirigami
    StatusNotifierItem
    DBusAddons
)

add_subdirectory(gui)

# ---- System integration files (the daemon binary is installed by the PKGBUILD) ----
install(FILES packaging/org.kde.wirearch.desktop
        DESTINATION ${KDE_INSTALL_APPDIR})
install(FILES packaging/dbus/tr.cebi.wirearch.conf
        DESTINATION /usr/share/dbus-1/system.d)
install(FILES packaging/dbus/tr.cebi.wirearch.service
        DESTINATION /usr/share/dbus-1/system-services)
install(FILES packaging/polkit/tr.cebi.wirearch.policy
        DESTINATION /usr/share/polkit-1/actions)
install(FILES packaging/systemd/wirearchd.service
        DESTINATION /usr/lib/systemd/system)
install(FILES packaging/systemd/wirearch.sysusers
        RENAME wirearch.conf
        DESTINATION /usr/lib/sysusers.d)
install(FILES packaging/modules-load.d/wirearch.conf
        DESTINATION /usr/lib/modules-load.d)

# ---- Plasma panel widget (optional) ----
find_package(Plasma ${KF6_MIN_VERSION} QUIET)
if(Plasma_FOUND)
    plasma_install_package(plasmoid/package tr.cebi.wirearch)
else()
    message(STATUS "libplasma not found; the Plasma panel widget will not be installed")
endif()

feature_summary(WHAT ALL FATAL_ON_MISSING_REQUIRED_PACKAGES)
