#!/usr/bin/env powershell

#
# Copyright 2017 yvt, all rights reserved.
#
# This source code is a part of Nightingales.
#

# TODO: invoke this automatically from build.rs?

# Source/tool paths
$NGSBaseSrcPath = Split-Path -Parent $PSCommandPath |
    % { Split-Path -Parent $_ } |
    % { Join-Path $_ "src" }
$NGSEnginePath = Split-Path -Parent $PSCommandPath |
    % { Split-Path -Parent $_ } |
    % { Split-Path -Parent $_ } |
    % { Split-Path -Parent $_ }
$NGSRustInteropGenPath = Split-Path -Parent $NGSEnginePath |
    % { Join-Path $_ "NGSRustInteropGen" }
$NGSRustInteropGenProjectPath = Join-Path $NGSRustInteropGenPath "NGSRustInteropGen.csproj"

# Output path
$OutputPath = Join-Path $NGSBaseSrcPath "interop.rs"

# External tools
$DotNet = "dotnet"

# Generate interop code
& $DotNet run -p $NGSRustInteropGenProjectPath -- -o $OutputPath
