cmake_minimum_required(VERSION 3.4)
project(Nightingales)

add_subdirectory(NGSCore)
add_subdirectory(NGSEngine)

include_external_msproject(NGSToolkit ${PROJECT_SOURCE_DIR}/NGSToolkit/NGSToolkit.csproj
    NGSEngineInterop)

include_external_msproject(NGSGameKit ${PROJECT_SOURCE_DIR}/NGSGameKit/NGSGameKit.csproj
    NGSToolkit
    NGSEngineInterop)

include_external_msproject(NGSGame ${PROJECT_SOURCE_DIR}/NGSGame/NGSGame.csproj
    NGSToolkit NGSGameKit)

include_external_msproject(NGSMain ${PROJECT_SOURCE_DIR}/NGSMain/NGSMain.csproj
    NGSGameKit NGSToolkit NGSEngineInterop)
