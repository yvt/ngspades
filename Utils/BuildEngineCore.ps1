#!/usr/bin/env pwsh

<#
.Synopsis
    Generates the full release build of the engine core.
.Description
    This script builds the engine core dylib for every supported processor feature level.
    This is accomplished by calling Cargo, the Rust programming language's package manager.

    After all required dylibs are built, they are deployed to the output directory, along with the
    engine loader configuration file, which contains the information required to locate the deployed
    dylibs.
.Parameter cargoTargetRootDirectory
    Specifies where the intermediate build output should be stored.
.Parameter outputDirectory
    Specifies the output directory where the dylibs of the engine core should be stored.
.Parameter cargoCommand
    Specifies the command string used to invoke Cargo.
#>

[CmdletBinding()]
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
        name = "generic"; suffix = "";
        rustFlags = ""; crates = @("ngsengine", "ngsloader")
    },
    @{
        name = "avx"; suffix = "-avx";
        rustFlags = "-Ctarget-feature=+sse3,+avx"; crates = @("ngsengine")
    },
    @{
        name = "avx2"; suffix = "-avx2";
        rustFlags = "-Ctarget-feature=+sse3,+avx,+avx2" ; crates = @("ngsengine")
    },
    @{
        name = "fma3"; suffix = "-fma3";
        rustFlags = "-Ctarget-feature=+sse3,+avx,+avx2,+fma"; crates = @("ngsengine")
    }
)

Write-Output "============= BuildEngineCore starting ================="
Write-Output "  Output directory = $outputDirectory"

foreach ($featureLevel in $featureLevels) {
    Write-Output ""
    Write-Output "--------------------------------------------------------"
    Write-Output ""
    Write-Output "# Building binaries for the feature level '$($featureLevel.name)'"

    $cargoTargetDirectory = Join-PAth $cargoTargetRootDirectory $featureLevel.name

    Write-Output ""
    Write-Output "  Cargo target directory = $cargoTargetDirectory"
    Write-Output ""

    [System.Environment]::SetEnvironmentVariable("RUSTFLAGS", $featureLevels.rustFlags)
    [System.Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", $cargoTargetDirectory)

    foreach ($crate in $featureLevel.crates) {
        Write-Output "## Building $crate"
        Set-Location -Path (Join-Path $engineCoreDirectory "src/$crate")
        &$cargoCommand build --release
        Write-Output ""
    }
}
Write-Output ""
Write-Output "--------------------------------------------------------"
Write-Output ""
Write-Output "# Collecting the outputs"
Write-Output ""

if ($IsWindows) {
    $dylibPrefix = ""
    $dylibSuffix = ".dll"
}
elseif ($IsMacOS) {
    $dylibPrefix = "lib"
    $dylibSuffix = ".dylib"
}
elseif ($IsLinux) {
    $dylibPrefix = "lib"
    $dylibSuffix = ".so"
}
else {
    Throw "Unknown platform - cannot determine the dylib file name.";
}

foreach ($featureLevel in $featureLevels) {
    $cargoTargetDirectory = Join-Path $cargoTargetRootDirectory $featureLevel.name

    foreach ($crate in $featureLevel.crates) {
        $builtDylibPath = Join-Path $cargoTargetDirectory "release" "$dylibPrefix$crate$dylibSuffix"
        $dylibOutputPath = Join-Path $outputDirectory "$dylibPrefix$crate$($featureLevel.suffix)$dylibSuffix"
        Write-Output "$builtDylibPath -> $dylibOutputPath"
        Copy-Item -Path $builtDylibPath $dylibOutputPath
    }
}

$loaderConfigPath = Join-Path $sourceDirectory "EngineCore" "NgsLoaderConfig.xml"
$loaderConfigOutputPath = Join-Path $outputDirectory "NgsLoaderConfig.xml"

Write-Output "$loaderConfigPath -> $loaderConfigOutputPath"
Copy-Item $loaderConfigPath $loaderConfigOutputPath

Write-Output ""
Write-Output "=============== BuildEngineCore done ==================="
