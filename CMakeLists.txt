cmake_minimum_required(VERSION 3.0)
project(Nightingales)

add_subdirectory(NGSCore)
add_subdirectory(NGSEngine)

include_external_msproject(NGSToolkit NGSToolkit/NGSToolkit.csproj
    PLATFORM "AnyCPU"
    NGSEngineInterop)

include_external_msproject(NGSGameKit NGSGameKit/NGSGameKit.csproj
    PLATFORM "AnyCPU"
    NGSToolkit
    NGSEngineInterop)

include_external_msproject(NGSGame NGSGame/NGSGame.csproj
    PLATFORM "AnyCPU"
    NGSToolkit NGSGameKit)

include_external_msproject(NGSMain NGSMain/NGSMain.csproj
    PLATFORM "AnyCPU"
    NGSGameKit NGSToolkit NGSEngineInterop)
