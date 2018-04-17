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

## Building

### Game Engine Core

    $ cd EngineCore/src/ngsengine
    $ cargo build
       Compiling ngsbase v0.1.0 (file:///home/who/where/EngineCore/src/ngsbase)
    ...
       Compiling ngsengine v0.1.0 (file:///home/who/where/EngineCore/src/ngsengine)
        Finished dev [unoptimized + debuginfo] target(s) in 23.45 secs

### Game Code

    $ cd Ngs.Application

    $ dotnet build
    Microsoft (R) Build Engine version 15.5.179.9764 for .NET Core
    Copyright (C) Microsoft Corporation. All rights reserved.
    ...
    Build succeeded.

## License

Copyright 2018 yvt, all rights reserved.
