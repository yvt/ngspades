#!/usr/bin/env pwsh

<#
.Synopsis
    Configures the NGS_ENGINE_PATH environment variable.
.Description
    This script sets the NGS_ENGINE_PATH environment variable to the engine core's build output
    directory (e.g., \EngineCore\target\debug).

    This script must be ran by dot sourcing as shown in the following example; otherwise it will
    have no effect.

        . ..\Utils\SetEnginePath.ps1

    Additionally, this script automatically creates a symbolic link of the loader configuration
    file (NgsLoaderConfig.xml) at the engine path. Note that creating a symlink requires the
    administrator privilege on earlier versions of Windows.

.Parameter cargoProfile
    Specifies the name of the Cargo build profile used to build the engine core. Specify "debug"
    or "release" here.
#>

[CmdletBinding()]
param(
    [string]$cargoProfile = "debug"
)

$sourceDirectory = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$engineCoreDirectory = Join-Path $sourceDirectory "EngineCore"
$cargoBuildDirectory = Join-Path $engineCoreDirectory "target/$cargoProfile"

Write-Output "NGS_ENGINE_PATH = $cargoBuildDirectory"
[System.Environment]::SetEnvironmentVariable("NGS_ENGINE_PATH", $cargoBuildDirectory)

# Create a symbolic link of the loader configuration file
$outLoaderConfigPath = Join-Path $cargoBuildDirectory "NgsLoaderConfig.xml"
$inLoaderConfigPath = Join-Path $sourceDirectory "EngineCore/NgsLoaderConfig.xml"

if (!(Test-Path $outLoaderConfigPath)) {
    Write-Output "Creating a link from $outLoaderConfigPath to $inLoaderConfigPath"

    if (!(Test-Path $inLoaderConfigPath)) {
        Throw "$inLoaderConfigPath was not found."
    }

    New-Item -Path $outLoaderConfigPath -ItemType SymbolicLink -Value $inLoaderConfigPath
}
