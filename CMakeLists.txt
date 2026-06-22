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
)

add_subdirectory(gui)

feature_summary(WHAT ALL FATAL_ON_MISSING_REQUIRED_PACKAGES)
