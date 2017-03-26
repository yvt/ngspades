cmake_minimum_required(VERSION 3.4)
project(NGSMain)

set(NGSMAIN_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(NGSMAIN_ENGINE_LIBARY_PATH ${CMAKE_CURRENT_BINARY_DIR}/../NGSEngine/libNGSEngine.so)
set(NGSMAIN_INTEROP_LIBARY_PATH ${CMAKE_CURRENT_BINARY_DIR}/../NGSEngine/NGSEngineInterop.dll)
configure_file(
    NGSMain.csproj.in
    ${CMAKE_CURRENT_BINARY_DIR}/NGSMain.csproj)

file(GLOB NGSMAIN_CS_FILES
    *.cs)

if(MSVC)
    include_external_msproject(NGSMain ${CMAKE_CURRENT_BINARY_DIR}/NGSMain.csproj
        NGSEngine NGSEngineInterop)
else()
    add_custom_target(NGSMainRestore
        dotnet restore
        WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR})
    add_custom_target(NGSMain ALL
        dotnet build
        DEPENDS NGSEngine NGSEngineInterop
        WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
        SOURCES ${NGSMAIN_CS_FILES})
endif()
