# SPDX-FileCopyrightText: 2026 ViewOS Project
# SPDX-License-Identifier: GPL-3.0-or-later
#
# FindEpoxy.cmake - Find libepoxy using pkg-config
#
# libepoxy does not provide a CMake config file. This module uses pkg-config
# to locate it, which is the standard method on Linux distributions.

find_package(PkgConfig REQUIRED)
pkg_check_modules(PC_EPOXY REQUIRED epoxy)

set(EPOXY_FOUND ${PC_EPOXY_FOUND})
set(EPOXY_INCLUDE_DIRS ${PC_EPOXY_INCLUDE_DIRS})
set(EPOXY_LIBRARIES ${PC_EPOXY_LIBRARIES})
set(EPOXY_VERSION ${PC_EPOXY_VERSION})

if(EPOXY_FOUND)
    if(NOT TARGET Epoxy::Epoxy)
        add_library(Epoxy::Epoxy INTERFACE IMPORTED)
        set_target_properties(Epoxy::Epoxy PROPERTIES
            INTERFACE_INCLUDE_DIRECTORIES "${EPOXY_INCLUDE_DIRS}"
            INTERFACE_LINK_LIBRARIES "${EPOXY_LIBRARIES}"
        )
    endif()
endif()

mark_as_advanced(EPOXY_INCLUDE_DIRS EPOXY_LIBRARIES EPOXY_VERSION)
