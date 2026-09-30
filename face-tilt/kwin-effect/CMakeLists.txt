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

find_package(ECM 6.10 REQUIRED NO_MODULE)
include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

# Pulls in Qt6, KF6 and defines the imported target KWin::kwin. It also pulls
# in KF6CoreAddons, which is where the kcoreaddons_add_plugin() macro used
# below comes from.
find_package(KWin 6.0 REQUIRED)

# Builds a MODULE library, sets LIBRARY_OUTPUT_DIRECTORY so the plugin can be
# run uninstalled, and installs it into
# ${KDE_INSTALL_PLUGINDIR}/kwin/effects/plugins, which is the directory
# EffectsHandler scans via KPluginMetaData::findPlugins().
kcoreaddons_add_plugin(viewosfacetilt
    SOURCES
        src/viewosfacetilt.cpp
        src/viewosfacetilt.h
        src/faceclient.cpp
        src/faceclient.h
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
