cmake_minimum_required(VERSION 2.6)
project(NGSEngine)

set(CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/CMake")

# Find C# compiler (required to create an interop assembly)
find_package(CSharp)
if(NOT CSHARP_FOUND)
    message(FATAL_ERROR "C# compiler was not found.")
endif()

# pyxpidl scripts
# (csinterop.py isn't a part of pyxpidl but placed in the same directory for convenience)
set(PYXPIDL_HEADER_COMMAND ${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/idl-parser/xpidl/header.py)
set(PYXPIDL_HEADER_FLAGS -I${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/base --cachedir=${PROJECT_BINARY_DIR}/../NGSCore/xpidlcache)

set(CSINTEROP_COMMAND ${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/idl-parser/xpidl/csinterop.py)
set(CSINTEROP_FLAGS ${PYXPIDL_HEADER_FLAGS})

include_directories(${PROJECT_BINARY_DIR}/../NGSCore/include)

# Preprocessor definitions for XPCOM library
add_definitions(-DMOZ_NO_MOZALLOC -DXPCOM_GLUE)
if(win32)
    add_definitions(-DXP_WIN)
else()
    add_definitions(-DXP_UNIX)
endif()

# IDL C++ header files
set(IDL_FILES
    ITestInterface.idl)

make_directory(${CMAKE_CURRENT_BINARY_DIR}/Xpcom/base)

foreach(CURRENT_IDL ${IDL_FILES})
    string(REGEX REPLACE "\\.idl$" ".h" CURRENT_H ${CURRENT_IDL})

    add_custom_command(
        OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H}
        COMMAND ${PYXPIDL_HEADER_COMMAND} ${PYXPIDL_HEADER_FLAGS} -o ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H} ${CMAKE_CURRENT_SOURCE_DIR}/${CURRENT_IDL}
        DEPENDS ${CURRENT_IDL})

    list(APPEND IDL_HEADER_FILES ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H})
endforeach()

include_directories(${PROJECT_BINARY_DIR})

# Target definition
add_library(NGSEngine MODULE
    Main.cpp
    ${IDL_HEADER_FILES})

set_property(TARGET NGSEngine PROPERTY CXX_STANDARD 11)
target_link_libraries(NGSEngine NGSCore)

# COM interop assembly generation
foreach(CURRENT_IDL ${IDL_FILES})
    string(REGEX REPLACE "\\.idl$" ".cs" CURRENT_CS ${CURRENT_IDL})

    add_custom_command(
        OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS}
        COMMAND ${CSINTEROP_COMMAND} ${CSINTEROP_FLAGS} -o ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS} ${CMAKE_CURRENT_SOURCE_DIR}/${CURRENT_IDL}
        DEPENDS ${CURRENT_IDL})

    list(APPEND IDL_CS_FILES ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS})
endforeach()

set(INTEROP_DLL ${CMAKE_CURRENT_BINARY_DIR}/NGSEngine.Interop.dll)
if(CSHARP_TYPE STREQUAL ".NET")
    # FIXME: are these quotation marks really needed?
    # FIXME: maybe path separator should be converted
    set(CS_FLAGS "/out:\"${INTEROP_DLL}\"" "/target:library")
    add_custom_target(NGSEngineInterop
        COMMAND ${CSHARP_COMPILER} ${IDL_CS_FILES}
        DEPENDS ${IDL_CS_FILES}
        SOURCES ${IDL_FILES})
else()
    set(CS_FLAGS "-out:\"${INTEROP_DLL}\"" "-target:library")
endif()
add_custom_target(NGSEngineInterop ALL
    COMMAND ${CSHARP_COMPILER} ${CS_FLAGS} ${IDL_CS_FILES}
    BYPRODUCTS ${INTEROP_DLL}
    DEPENDS ${IDL_CS_FILES}
    COMMENT "Creating COM interop assembly"
    SOURCES ${IDL_FILES})
