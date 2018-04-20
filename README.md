Nightingales
============

## Prerequisite

The following softwares must be installed to build this project.

- [Rust] 1.26 or later. The nightly build toolchain must be installed and selected using `rustup install nightly` and `rustup default nightly`.
- [.NET Core] 2.0 or later
- [LunarG Vulkan SDK] 1.0 or later. [glslang], which is distributed as a part of it, must be in `$PATH` to build ZanGFX and the rendering engine. 

[Rust]: https://www.rust-lang.org/en-US/
[.NET Core]: https://www.microsoft.com/net/download/
[LunarG Vulkan SDK]: https://www.lunarg.com/vulkan-sdk/
[glslang]: https://github.com/KhronosGroup/glslang

## Building for development

### Game Engine Core

    $ cd EngineCore/src/ngsengine
    $ cargo build
       Compiling ngsbase v0.1.0 (file:///home/who/where/EngineCore/src/ngsbase)
    ...
       Compiling ngsengine v0.1.0 (file:///home/who/where/EngineCore/src/ngsengine)
        Finished dev [unoptimized + debuginfo] target(s) in 23.45 secs

### Game Engine Loader Helper

    $ cd EngineCore/src/ngsloader
    $ cargo build

### Game Code

    $ cd Ngs.Application

    $ dotnet build
    Microsoft (R) Build Engine version 15.5.179.9764 for .NET Core
    Copyright (C) Microsoft Corporation. All rights reserved.
    ...
    Build succeeded.

    $ export NGS_ENGINE_PATH=$(pwd)
    $ ln -s ../EngineCore/target/debug/libngsengine.dylib
    $ ln -s ../EngineCore/target/debug/libngsloader.dylib
    $ dotnet run
    Checking if the engine core was successfully loaded
    $

## Building as a stand-alone application

We utilize .NET Core's [Self-contained deployment](https://docs.microsoft.com/en-us/dotnet/core/deploying/#self-contained-deployments-scd) feature to create a stand-alone application package. The result is an executable along with a bunch of dependent libraries (which mostly originate from the .NET Core standard library). To reduce the size of it further, we execute [.NET IL Linker](https://github.com/dotnet/core/blob/master/samples/linker-instructions.md) as a part of the build pipeline.

    $ cd Ngs.Application

    # Create a self-contained deployment
    $ dotnet publish -c Release -r osx-x64 -o ../Derived/Scd/

    # This process might left some garbage files that we can delete safely
    $ rm -R _ Optimize

    # Copy native libraries
    $ cp LoaderConfig.xml ../EngineCore/target/debug/libngs*.dylib ../Derived/Scd/

## License

Copyright 2018 yvt, all rights reserved.
