cmake_minimum_required(VERSION 3.4)
project(NGSEngine)

set(CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/CMake" "${CMAKE_CURRENT_SOURCE_DIR}/../NGSCore/CMake")

# Find C# compiler (required to create an interop assembly)
find_package(CSharp)
if(NOT CSHARP_FOUND)
    message(FATAL_ERROR "C# compiler was not found.")
endif()

include_directories(${PROJECT_SOURCE_DIR})

# pyxpidl scripts
# (csinterop.py isn't a part of pyxpidl but placed in the same directory for convenience)
set(PYXPIDL_HEADER_COMMAND ${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/idl-parser/xpidl/header.py)
set(PYXPIDL_HEADER_FLAGS -I${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/base -I${PROJECT_SOURCE_DIR}/../NGSCore/Utils -I${CMAKE_CURRENT_SOURCE_DIR} --cachedir=${PROJECT_BINARY_DIR}/../NGSCore/xpidlcache)

set(CSINTEROP_COMMAND ${PROJECT_SOURCE_DIR}/../NGSCore/Xpcom/idl-parser/xpidl/csinterop.py)
set(CSINTEROP_FLAGS ${PYXPIDL_HEADER_FLAGS} -c ${PROJECT_SOURCE_DIR}/Interop/CSInteropConfig.json)

include_directories(${PROJECT_BINARY_DIR}/../NGSCore/Headers)

# Preprocessor definitions for XPCOM library
add_definitions(-DMOZ_NO_MOZALLOC -DXPCOM_GLUE)
if(win32)
    add_definitions(-DXP_WIN)
else()
    add_definitions(-DXP_UNIX)
endif()

# C++ source files
file(GLOB BACKEND_FILES
    Backend/Common/BlendFactor.h
    Backend/Common/BlendOperation.h
    Backend/Common/ColorWriteMask.h
    Backend/Common/GPUFunction.h
    Backend/Common/GraphicsBackend.cpp
    Backend/Common/GraphicsBackend.h
    Backend/Common/PixelFormat.h
    Backend/Common/RenderPass.h
    Backend/Common/RenderPassAttachmentDescriptor.cpp
    Backend/Common/RenderPassAttachmentDescriptor.h
    Backend/Common/RenderPassDescriptor.cpp
    Backend/Common/RenderPassDescriptor.h
    Backend/Common/RenderPipelineAttachmentDescriptor.cpp
    Backend/Common/RenderPipelineAttachmentDescriptor.h
    Backend/Common/RenderPipelineDescriptor.cpp
    Backend/Common/RenderPipelineDescriptor.h
    Backend/Common/RenderPipelineState.h
    Backend/Common/Texture.h
    Backend/Common/Viewport.h
    Backend/SDL/ImportSDL.h
    Backend/SDL/SDLViewport.cpp
    Backend/SDL/SDLViewport.h)

if(APPLE)
    file(GLOB BACKEND_METAL_FILES
        Backend/Metal/MetalGraphicsBackend.h
        Backend/Metal/MetalGraphicsBackend.mm
        Backend/Metal/MetalGraphicsBackendPrivate.h
        Backend/Metal/NGSMetalView.h
        Backend/Metal/NGSMetalView.mm)
    set(BACKEND_FILES ${BACKEND_FILES} ${BACKEND_METAL_FILES})

    find_library(METAL_LIBRARY Metal)
    mark_as_advanced(METAL_LIBRARY)
    list(APPEND EXTRA_LIBS ${METAL_LIBRARY})

    find_library(METALKIT_LIBRARY MetalKit)
    mark_as_advanced(METALKIT_LIBRARY)
    list(APPEND EXTRA_LIBS ${METALKIT_LIBRARY})
else()
    message(FATAL_ERROR "Unsupported platform")
endif()

file(GLOB UTILS_FILES
    Utils/ComException.cpp
    Utils/ComException.h
    Utils/Geometry.cpp
    Utils/Geometry.h
    Utils/Heap.cpp
    Utils/Heap.h
    Utils/SegmentedHeap.cpp
    Utils/SegmentedHeap.h
    Utils/UninitializedMemoryBlock.cpp
    Utils/UninitializedMemoryBlock.h)
file(GLOB WORLD_FILES
    World/TerrainVoxelInfo.h
    World/VoxelTerrain.cpp
    World/VoxelTerrain.h)

set(CXX_FILES Main.cpp ${BACKEND_FILES} ${UTILS_FILES} ${WORLD_FILES})

# IDL C++ header files
set(IDL_FILES
    ITestInterface.idl
    IEngine.idl
    Backend/Common/IKeyboardEvent.idl
    Backend/Common/IMouseEvent.idl
    Backend/Common/ITextCompositionEvent.idl
    Backend/Common/ITextInputEvent.idl
    Backend/Common/IViewport.idl
    Backend/Common/IViewportListener.idl
    World/IVoxelTerrain.idl
    Interop/Types.idl)

make_directory(${CMAKE_CURRENT_BINARY_DIR}/Headers)

foreach(CURRENT_IDL ${IDL_FILES})
    string(REGEX REPLACE "\\.idl$" ".h" CURRENT_H ${CURRENT_IDL})
    get_filename_component(CURRENT_H ${CURRENT_H} NAME)
    string(CONCAT CURRENT_H "Headers/" ${CURRENT_H})

    add_custom_command(
        OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H}
        COMMAND ${PYXPIDL_HEADER_COMMAND} ${PYXPIDL_HEADER_FLAGS} -o ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H} ${CMAKE_CURRENT_SOURCE_DIR}/${CURRENT_IDL}
        DEPENDS ${CURRENT_IDL})

    list(APPEND IDL_HEADER_FILES ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_H})
endforeach()

include_directories(${PROJECT_BINARY_DIR}/Headers)

# Target definition
add_library(NGSEngine MODULE
    ${IDL_HEADER_FILES}
    ${CXX_FILES})

set_property(TARGET NGSEngine PROPERTY CXX_STANDARD 14)
target_link_libraries(NGSEngine NGSCore ${EXTRA_LIBS})

# Use C++17
include(EnableCxx17)
target_enable_cxx17(NGSEngine)

# COM interop assembly generation
foreach(CURRENT_IDL ${IDL_FILES})
    string(REGEX REPLACE "\\.idl$" ".cs" CURRENT_CS ${CURRENT_IDL})
    get_filename_component(CURRENT_CS ${CURRENT_CS} NAME)
    string(CONCAT CURRENT_CS "InteropAssembly/" ${CURRENT_CS})

    add_custom_command(
        OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS}
        COMMAND ${CSINTEROP_COMMAND} ${CSINTEROP_FLAGS} -o ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS} ${CMAKE_CURRENT_SOURCE_DIR}/${CURRENT_IDL}
        DEPENDS ${CURRENT_IDL})

    list(APPEND IDL_CS_FILES ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS})
endforeach()

file(GLOB AUX_CS_FILES
    Interop/Geometry.cs
    Interop/CSTypes.cs)

foreach(CURRENT_CS ${AUX_CS_FILES})
    get_filename_component(CURRENT_CS_NEW ${CURRENT_CS} NAME)
    string(CONCAT CURRENT_CS_NEW "InteropAssembly/" ${CURRENT_CS_NEW})
    configure_file(
        ${CURRENT_CS}
        ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS_NEW}
        COPYONLY)

    list(APPEND AUX_CS_FILES_COPIED ${CMAKE_CURRENT_BINARY_DIR}/${CURRENT_CS_NEW})
endforeach()

set(INTEROP_DLL ${CMAKE_CURRENT_BINARY_DIR}/NGSEngine.Interop.dll)
if(CSHARP_TYPE STREQUAL ".NET")
    set(CS_FLAGS "/out:${INTEROP_DLL}" "/target:library")
else()
    set(CS_FLAGS "-out:${INTEROP_DLL}" "-target:library")
endif()
add_custom_target(NGSEngineInterop ALL
    COMMAND ${CSHARP_COMPILER} ${CS_FLAGS} ${AUX_CS_FILES_COPIED} ${IDL_CS_FILES}
    BYPRODUCTS ${INTEROP_DLL}
    DEPENDS ${IDL_CS_FILES} ${AUX_CS_FILES_COPIED}
	WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/InteropAssembly
    COMMENT "Creating COM interop assembly"
    SOURCES ${IDL_FILES} ${AUX_CS_FILES})
