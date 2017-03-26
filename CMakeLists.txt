cmake_minimum_required(VERSION 3.4)
project(Nightingales)

add_subdirectory(NGSCore)
add_subdirectory(NGSEngine)
add_subdirectory(NGSMain)

if(NOT MSVC)
add_custom_target(restore DEPENDS NGSMainRestore)
endif()
