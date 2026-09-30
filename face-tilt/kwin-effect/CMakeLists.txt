# SPDX-FileCopyrightText: 2026 ViewOS Project
# SPDX-License-Identifier: GPL-3.0-or-later

cmake_minimum_required(VERSION 3.22)

project(viewosfacetilt LANGUAGES CXX)

# KWin 6.3 is built as C++23; 20 is the lowest standard that expresses
# everything used here while staying available on older toolchains.
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# The effect is a loadable module, not something to be linked into anything.
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# libepoxy does not ship a CMake config file, but KWinConfig.cmake calls
# find_dependency(epoxy). Provide a config file in a local directory and add
# it to CMAKE_PREFIX_PATH so find_package(epoxy) finds it.
# The config file must be at <prefix>/share/cmake/epoxy/epoxyConfig.cmake
# for CMake's standard search to find it.
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/epoxy_prefix/share/cmake/epoxy")
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/epoxyConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/epoxy_prefix/share/cmake/epoxy/epoxyConfig.cmake"
    @ONLY
)
list(APPEND CMAKE_PREFIX_PATH "${CMAKE_CURRENT_BINARY_DIR}/epoxy_prefix")

find_package(ECM 6.10 REQUIRED NO_MODULE)

# ECM 6.x places the KDE helper modules under kde-modules/, and they
# are NOT added to CMAKE_MODULE_PATH by ECMConfig.cmake. Include them
# explicitly from ECM_KDE_MODULE_DIR, which ECMConfig.cmake sets to
# <prefix>/share/ECM/kde-modules/. Guard against ECM < 5.82 where the
# layout was different and the modules were in ECM_MODULE_DIR directly.
if(EXISTS "${ECM_KDE_MODULE_DIR}/KDEInstallDirs.cmake")
    include("${ECM_KDE_MODULE_DIR}/KDEInstallDirs.cmake")
    include("${ECM_KDE_MODULE_DIR}/KDECMakeSettings.cmake")
    include("${ECM_KDE_MODULE_DIR}/KDECompilerSettings.cmake" NO_POLICY_SCOPE)
elseif(EXISTS "${ECM_MODULE_DIR}/kde-modules/KDEInstallDirs.cmake")
    include("${ECM_MODULE_DIR}/kde-modules/KDEInstallDirs.cmake")
    include("${ECM_MODULE_DIR}/kde-modules/KDECMakeSettings.cmake")
    include("${ECM_MODULE_DIR}/kde-modules/KDECompilerSettings.cmake" NO_POLICY_SCOPE)
else()
    include(KDEInstallDirs)
    include(KDECMakeSettings)
    include(KDECompilerSettings NO_POLICY_SCOPE)
endif()

# Pulls in Qt6, KF6 and defines the imported target KWin::kwin.
# It also pulls in KF6CoreAddons, which provides kcoreaddons_add_plugin().
# KWinTargets.cmake requires Qt6::Widgets in its INTERFACE_LINK_LIBRARIES.
find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick Widgets)
find_package(KWin 6.0 REQUIRED)

# Builds a MODULE library, sets LIBRARY_OUTPUT_DIRECTORY so the plugin can be
# run uninstalled, and installs it into
# ${KDE_INSTALL_PLUGINDIR}/kwin/effects/plugins, which is the directory
# EffectsHandler scans via KPluginMetaData::findPlugins().
kcoreaddons_add_plugin(viewosfacetilt
    METADATA_JSON metadata.json
    SOURCES
        src/viewosfacetilt.cpp
        src/viewosfacetilt.h
        src/faceclient.cpp
        src/faceclient.h
        viewosfacetilt.qrc
    INSTALL_NAMESPACE "kwin/effects/plugins"
)

target_link_libraries(viewosfacetilt PRIVATE
    KWin::kwin
    Qt6::Core
    Qt6::Network
)

# metadata.json is embedded into the shared object by the
# K_PLUGIN_FACTORY_WITH_JSON declaration in viewosfacetilt.cpp, which is where
# QPluginLoader::metaData() reads it from. It is therefore not installed as a
# separate file.