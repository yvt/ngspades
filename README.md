Nightingales
============

## Project structure

### Shared assets

- `Assets` contains a font resource shared by some projects and unit tests.

### Game engine

- `EngineCore` contains the Rust part of the engine and its dependent crates (libraries).
    - `ngsengine` is the engine core. It is compiled as a dynamic library which is loaded by the .NET part of the engine at runtime.
    - `ngsloader` is another dynamic library required by the game engine. The engine loader uses this library to examine the capability of the processor on which it runs and chooses the appropriate version of the engine core.

- `Ngs.Interop` is a .NET library for the NgsCOM (an ABI based on Component Object Model) interoperation. This library is essential for communication between the engine core and the .NET part of this engine.

- `Ngs.Engine.Core` is a .NET library that serves the following purposes:
    - It defines NgsCOM interfaces (a kind of contract between softwares). Most of them are implemented by the engine core (look for crates whose names ending with `_com`) and consumed by the .NET part. There are some opposites cases.
    - It provides basic data types such as `IntVector3` and `Rgba`.
    - It provides the engine core loader, which locates and loads `ngsengine`, the engine core. This process is assisted by `ngsloader`, which is another native dynamic library.
    - This project contains [a separate developer's documentation](./Ngs.Engine.Core/Readme.md.html).

- `Ngs.RustInteropGen` is a .NET application that generates Rust code from the NgsCOM interface definition in `Ngs.Engine.Core`. The build script of the `ngsbase` crate calls this application during build. You can run this application directly to see its output.

- `Ngs.Engine.Framework` is a framework that provides common functionalities for building applications based on the engine. This project contains [a separate developer's documentation](./Ngs.Engine.Framework/Readme.md.html).

Some .NET projects are accompanied by xUnit test projects, which can be identified by the suffix `.Tests`.

### Development tools

- `Ngs.Performance` is a .NET application based on [Benchmark.NET], used for micro-benchmarking the performance of the game engine.

- `Utils/Cloc.sh` calls [loc], a blazing-fast Rust port of the popular CLOC utility. As the name implies, it counts lines of code in the source tree.

- `Utils/Monodoc.sh` calls Monodoc to generate HTML documentations of .NET assemblies.

[Benchmark.NET]: http://benchmarkdotnet.org
[loc]: https://crates.io/crates/loc

### Applications

 - `Ngs.Application` doesn't have an exact purpose. I primarily use this project to test the new functionalities of the engine.
 - `Ngs.Editor`, also known as Nightingales Editor, is an application used for producing assets and game levels.

## Prerequisite

The following softwares must be installed to build this project.

- [Rust] 1.26 or later. The nightly build toolchain must be installed and selected using `rustup install nightly` and `rustup default nightly`.
- [.NET Core] 2.1 or later
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

    $ export NGS_ENGINE_PATH=$(pwd)/EngineCore/target/debug
    $ pushd $NGS_ENGINE_PATH
    $ ln -s ../../../Ngs.Application/LoaderConfig.xml
    $ popd
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
