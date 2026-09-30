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

# libepoxy does not ship a CMake config file, but KWinTargets.cmake expects
# an imported target named epoxy::epoxy in its INTERFACE_LINK_LIBRARIES.
# Create it directly using execute_process to run pkg-config at configure time,
# so the target exists before find_package(KWin) is called.
execute_process(
    COMMAND pkg-config --cflags --libs epoxy
    OUTPUT_VARIABLE EPOXY_PKG_OUTPUT
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE EPOXY_PKG_RESULT
)
if(EPOXY_PKG_RESULT)
    message(FATAL_ERROR "pkg-config failed to find epoxy: ${EPOXY_PKG_RESULT}")
endif()

# Parse the output into include dirs and libraries
# pkg-config output looks like: -I/usr/include -L/usr/lib -lepoxy
string(REGEX MATCHALL "-I[^ ]+" EPOXY_INCLUDE_FLAGS "${EPOXY_PKG_OUTPUT}")
string(REGEX MATCHALL "-l[^ ]+" EPOXY_LIB_FLAGS "${EPOXY_PKG_OUTPUT}")
string(REGEX MATCHALL "-L[^ ]+" EPOXY_LIBDIR_FLAGS "${EPOXY_PKG_OUTPUT}")

# Convert -I/path to /path for INTERFACE_INCLUDE_DIRECTORIES
set(EPOXY_INCLUDE_DIRS "")
foreach(flag ${EPOXY_INCLUDE_FLAGS})
    string(REGEX REPLACE "^-I" "" path "${flag}")
    list(APPEND EPOXY_INCLUDE_DIRS "${path}")
endforeach()

# Convert -lname to name for INTERFACE_LINK_LIBRARIES
set(EPOXY_LIBRARIES "")
foreach(flag ${EPOXY_LIB_FLAGS})
    string(REGEX REPLACE "^-l" "" lib "${flag}")
    list(APPEND EPOXY_LIBRARIES "${lib}")
endforeach()

# Convert -L/path for link directories (not usually needed for imported targets)
set(EPOXY_LIBRARY_DIRS "")
foreach(flag ${EPOXY_LIBDIR_FLAGS})
    string(REGEX REPLACE "^-L" "" path "${flag}")
    list(APPEND EPOXY_LIBRARY_DIRS "${path}")
endforeach()

# Create the imported target that KWinTargets.cmake expects
if(NOT TARGET epoxy::epoxy)
    add_library(epoxy::epoxy INTERFACE IMPORTED)
    set_target_properties(epoxy::epoxy PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${EPOXY_INCLUDE_DIRS}"
        INTERFACE_LINK_LIBRARIES "${EPOXY_LIBRARIES}"
        INTERFACE_LINK_DIRECTORIES "${EPOXY_LIBRARY_DIRS}"
    )
endif()

# Mark epoxy as found so find_dependency(epoxy) in KWinConfig.cmake is satisfied
set(epoxy_FOUND TRUE)

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