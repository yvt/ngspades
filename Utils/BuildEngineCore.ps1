#!/usr/bin/env pwsh

# This PowerShell script compiles the engine core for each processor feature level.

param(
    [string]$cargoTargetRootDirectory,
    [string]$outputDirectory,
    [string]$cargoCommand = "cargo"
)

$sourceDirectory = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$engineCoreDirectory = Join-Path $sourceDirectory "EngineCore"

# Default output directories
if ($cargoTargetRootDirectory -eq "") {
    $cargoTargetRootDirectory = Join-Path $sourceDirectory "Derived/EngineCore"
}

if ($outputDirectory -eq "") {
    $outputDirectory = Join-Path $sourceDirectory "Derived/EngineCore"
}

$featureLevels = @(
    @{
        name="generic"; suffix="";
        rustFlags=""; crates=@("ngsengine", "ngsloader")
    },
    @{
        name="avx"; suffix="-avx";
        rustFlags="-Ctarget-feature=+sse3,+avx"; crates=@("ngsengine")
    },
    @{
        name="avx2"; suffix="-avx2";
        rustFlags="-Ctarget-feature=+sse3,+avx,+avx2" ; crates=@("ngsengine")
    },
    @{
        name="fma3"; suffix="-fma3";
        rustFlags="-Ctarget-feature=+sse3,+avx,+avx2,+fma"; crates=@("ngsengine")
    }
)

Echo "============= BuildEngineCore starting ================="
Echo "  Output directory = $outputDirectory"

forEach ($featureLevel in $featureLevels) {
    Echo ""
    Echo "--------------------------------------------------------"
    Echo ""
    Echo "# Building binaries for the feature level '$($featureLevel.name)'"

    $cargoTargetDirectory = Join-PAth $cargoTargetRootDirectory $featureLevel.name

    Echo ""
    Echo "  Cargo target directory = $cargoTargetDirectory"
    Echo ""

    [System.Environment]::SetEnvironmentVariable("RUSTFLAGS", $featureLevels.rustFlags)
    [System.Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", $cargoTargetDirectory)

    forEach ($crate in $featureLevel.crates) {
        Echo "## Building $crate"
        Set-Location -Path (Join-Path $engineCoreDirectory "src/$crate")
        &$cargoCommand build --release
        Echo ""
    }
}
Echo ""
Echo "--------------------------------------------------------"
Echo ""
Echo "# Collecting the outputs"
Echo ""

if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows))
{
    $dylibPrefix = ""
    $dylibSuffix = ".dll"
}
elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::OSX))
{
    $dylibPrefix = "lib"
    $dylibSuffix = ".dylib"
}
elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Linux))
{
    $dylibPrefix = "lib"
    $dylibSuffix = ".so"
} else {
    Throw "Unknown platform - cannot determine the dylib file name.";
}

forEach ($featureLevel in $featureLevels) {
    $cargoTargetDirectory = Join-PAth $cargoTargetRootDirectory $featureLevel.name

    forEach ($crate in $featureLevel.crates) {
        $builtDylibPath = Join-Path $cargoTargetDirectory "release" "$dylibPrefix$crate$dylibSuffix"
        $dylibOutputPath = Join-PAth $outputDirectory "$dylibPrefix$crate$($featureLevel.suffix)$dylibSuffix"
        Echo "$builtDylibPath -> $dylibOutputPath"
        Copy-Item -Path $builtDylibPath $dylibOutputPath
    }
}

Echo ""
Echo "=============== BuildEngineCore done ==================="
