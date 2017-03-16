cmake_minimum_required(VERSION 2.6)
project(NGSCore)

set(PYXPIDL_HEADER_COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/idl-parser/xpidl/header.py)
set(PYXPIDL_HEADER_FLAGS -I${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/base --cachedir=${CMAKE_CURRENT_BINARY_DIR}/xpidlcache)

add_definitions(-DMOZ_NO_MOZALLOC -DXPCOM_GLUE)
if(win32)
    add_definitions(-DXP_WIN)
else()
    add_definitions(-DXP_UNIX)
endif()

file(GLOB XPCOM_BASE_FILES Xpcom/base/*.cpp Xpcom/base/*.h)
file(GLOB XPCOM_BUILD_FILES Xpcom/build/*.cpp Xpcom/build/*.h)
file(GLOB MFBT_FILES Mfbt/*.cpp Mfbt/*.h)

include_directories(${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/base)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/build)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/reflect)

# Public header files
file(GLOB PUBLIC_HEADER_FILES
    Xpcom/base/*.h
    Xpcom/build/nsXPCOM.h
    Xpcom/build/nsXPCOMCID.h
    Xpcom/build/nsXPCOMCIDInternal.h)
file(GLOB MOZILLA_PUBLIC_HEADER_FILES
    Mfbt/*.h)
foreach(CURRENT_H ${PUBLIC_HEADER_FILES})
    get_filename_component(CURRENT_H_BASENAME ${CURRENT_H} NAME)
    configure_file(
        ${CURRENT_H}
        ${CMAKE_CURRENT_BINARY_DIR}/Headers/${CURRENT_H_BASENAME}
        COPYONLY)
endforeach()
foreach(CURRENT_H ${MOZILLA_PUBLIC_HEADER_FILES})
    get_filename_component(CURRENT_H_BASENAME ${CURRENT_H} NAME)
    configure_file(
        ${CURRENT_H}
        ${CMAKE_CURRENT_BINARY_DIR}/Headers/mozilla/${CURRENT_H_BASENAME}
        COPYONLY)
endforeach()

include_directories(${CMAKE_CURRENT_BINARY_DIR}/Headers)

# Config files
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/xpcom-config.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/Headers/xpcom-config.h)
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/Xpcom/xpcom-private.h.in
    ${CMAKE_CURRENT_BINARY_DIR}/Headers/xpcom-private.h)

# IDL C++ header files
set(XPCOM_IDL_FILES
    Xpcom/base/nsrootidl.idl
    Xpcom/base/nsISupports.idl
    Xpcom/base/nsIMemory.idl
    Xpcom/base/nsIDebug2.idl)

make_directory(${CMAKE_CURRENT_BINARY_DIR}/Xpcom/base)

foreach(CURRENT_IDL ${XPCOM_IDL_FILES})
    string(REGEX REPLACE "\\.idl$" ".h" CURRENT_H ${CURRENT_IDL})
    get_filename_component(CURRENT_H ${CURRENT_H} NAME)
    string(CONCAT CURRENT_H "Headers/" ${CURRENT_H})

    add_custom_command(
        OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H}
        COMMAND ${PYXPIDL_HEADER_COMMAND} ${PYXPIDL_HEADER_FLAGS} -o ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H} ${CMAKE_CURRENT_SOURCE_DIR}/${CURRENT_IDL}
        DEPENDS ${CURRENT_IDL})
    list(APPEND XPCOM_IDL_H_FILES ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H})
endforeach()

# Create library
add_library(NGSCore STATIC
    ${XPCOM_BASE_FILES}
    ${XPCOM_BUILD_FILES}
    ${XPCOM_IDL_H_FILES}
    ${MFBT_FILES})
set_property(TARGET NGSCore PROPERTY CXX_STANDARD 11)

