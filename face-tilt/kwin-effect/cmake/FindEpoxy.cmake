# SPDX-FileCopyrightText: 2026 ViewOS Project
# SPDX-License-Identifier: GPL-3.0-or-later
#
# FindEpoxy.cmake - Find libepoxy using pkg-config
#
# This module is placed in CMAKE_MODULE_PATH so find_package(epoxy) finds it
# when KWinConfig.cmake calls find_dependency(epoxy).

# Use execute_process to run pkg-config at configure time
execute_process(
    COMMAND pkg-config --cflags --libs epoxy
    OUTPUT_VARIABLE _EPOXY_PKG_OUTPUT
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _EPOXY_PKG_RESULT
)
if(_EPOXY_PKG_RESULT)
    message(FATAL_ERROR "pkg-config failed to find epoxy: ${_EPOXY_PKG_RESULT}")
endif()

# Parse the output into include dirs and libraries
string(REGEX MATCHALL "-I[^ ]+" _EPOXY_INCLUDE_FLAGS "${_EPOXY_PKG_OUTPUT}")
string(REGEX MATCHALL "-l[^ ]+" _EPOXY_LIB_FLAGS "${_EPOXY_PKG_OUTPUT}")
string(REGEX MATCHALL "-L[^ ]+" _EPOXY_LIBDIR_FLAGS "${_EPOXY_PKG_OUTPUT}")

# Convert -I/path to /path for INTERFACE_INCLUDE_DIRECTORIES
set(EPOXY_INCLUDE_DIRS "")
foreach(flag ${_EPOXY_INCLUDE_FLAGS})
    string(REGEX REPLACE "^-I" "" path "${flag}")
    list(APPEND EPOXY_INCLUDE_DIRS "${path}")
endforeach()

# Convert -lname to name for INTERFACE_LINK_LIBRARIES
set(EPOXY_LIBRARIES "")
foreach(flag ${_EPOXY_LIB_FLAGS})
    string(REGEX REPLACE "^-l" "" lib "${flag}")
    list(APPEND EPOXY_LIBRARIES "${lib}")
endforeach()

# Convert -L/path for link directories
set(EPOXY_LIBRARY_DIRS "")
foreach(flag ${_EPOXY_LIBDIR_FLAGS})
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

# Standard find_package variables
set(EPOXY_FOUND TRUE)
set(EPOXY_VERSION "1.5.10")  # libepoxy version in Trixie
set(EPOXY_INCLUDE_DIRS "${EPOXY_INCLUDE_DIRS}")
set(EPOXY_LIBRARIES "${EPOXY_LIBRARIES}")

# Mark as found for find_dependency
set(epoxy_FOUND TRUE)
set(epoxy_VERSION "${EPOXY_VERSION}")
set(epoxy_INCLUDE_DIRS "${EPOXY_INCLUDE_DIRS}")
set(epoxy_LIBRARIES "${EPOXY_LIBRARIES}")

mark_as_advanced(EPOXY_INCLUDE_DIRS EPOXY_LIBRARIES EPOXY_VERSION)