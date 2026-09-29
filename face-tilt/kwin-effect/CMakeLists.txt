# ViewOS Face Tilt KWin Effect
# Build with: cmake -B build && cmake --build build

cmake_minimum_required(VERSION 3.16)
project(viewosfacetilt LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# KWin effects need these
find_package(ECM 5.80 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH} ${ECM_KDE_MODULE_DIR})

find_package(KF6Config REQUIRED)
find_package(KF6CoreAddons REQUIRED)
find_package(KF6WindowSystem REQUIRED)
find_package(KF6I18n REQUIRED)

# KWin private headers
find_package(KWin 6.0 REQUIRED)

# Qt6
find_package(Qt6 REQUIRED COMPONENTS Core Gui DBus)

# Our effect
set(EFFECT_NAME "viewosfacetilt")

add_library(${EFFECT_NAME} MODULE
    src/viewosfacetilt.cpp
    src/viewosfacetilt.h
    src/faceclient.cpp
    src/faceclient.h
)

target_link_libraries(${EFFECT_NAME} PRIVATE
    KF6::ConfigCore
    KF6::CoreAddons
    KF6::WindowSystem
    KWin::Effects
    Qt6::Core
    Qt6::Gui
    Qt6::DBus
)

# Install as KWin effect
install(TARGETS ${EFFECT_NAME}
    PLUGIN_DESTINATION ${KWIN_PLUGIN_INSTALL_DIR}
)

# Metadata
ecm_qt6_declare_metatypes(src/viewosfacetilt.h)

# Install metadata
install(FILES metadata.json
    DESTINATION ${KWIN_EFFECTS_INSTALL_DIR}/${EFFECT_NAME}
)