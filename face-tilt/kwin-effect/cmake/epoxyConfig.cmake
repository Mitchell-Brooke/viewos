# SPDX-FileCopyrightText: 2026 ViewOS Project
# SPDX-License-Identifier: GPL-3.0-or-later
#
# epoxyConfig.cmake - Fake config file for libepoxy
#
# libepoxy doesn't provide a CMake config file. This file is placed at a known
# location and epoxy_DIR is set to point to it, so that find_dependency(epoxy)
# in KWinConfig.cmake succeeds.
#
# KWinTargets.cmake expects a target named `epoxy::epoxy` (lowercase), not
# `Epoxy::Epoxy`.

# Use pkg-config to get the actual flags
find_package(PkgConfig REQUIRED)
pkg_check_modules(PC_EPOXY REQUIRED epoxy)

# Create the imported target that KWin expects
if(NOT TARGET epoxy::epoxy)
    add_library(epoxy::epoxy INTERFACE IMPORTED)
    set_target_properties(epoxy::epoxy PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${PC_EPOXY_INCLUDE_DIRS}"
        INTERFACE_LINK_LIBRARIES "${PC_EPOXY_LIBRARIES}"
        INTERFACE_COMPILE_OPTIONS "${PC_EPOXY_CFLAGS_OTHER}"
    )
endif()

# Provide the standard config-file variables
set(EPOXY_FOUND TRUE)
set(EPOXY_VERSION "${PC_EPOXY_VERSION}")
set(EPOXY_INCLUDE_DIRS "${PC_EPOXY_INCLUDE_DIRS}")
set(EPOXY_LIBRARIES "${PC_EPOXY_LIBRARIES}")
set(EPOXY_DEFINITIONS "${PC_EPOXY_CFLAGS_OTHER}")

# Compatibility with find_dependency expectations
set(epoxy_FOUND ${EPOXY_FOUND})
set(epoxy_VERSION ${EPOXY_VERSION})
set(epoxy_INCLUDE_DIRS ${EPOXY_INCLUDE_DIRS})
set(epoxy_LIBRARIES ${EPOXY_LIBRARIES})