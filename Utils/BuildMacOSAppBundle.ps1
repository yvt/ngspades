#!/usr/bin/env pwsh

<#
.Synopsis
    Publishes a .NET application as a standard macOS application.
.Description

.Parameter projectDirectory
    Specifies the directory path where the .NET application project is located. The directory must
    contain a subdirectory named MacOSBundle. Defaults to the current directory.
.Parameter outputDirectory
    Specifies the path of the output directory where the application bundle is generated.
.Parameter dotnetCommand
    Specifies the command string used to invoke .NET Core command-line utility.
.Parameter buildConfiguration
    Specifies the configuration name used to build the .NET project.
.Parameter noDeployEngineCore
    Skips copying of the engine core dylibs.
.Parameter engineCoreDirectory
    Specifies the directory path where the engine core dylib and the engine loader configuration
    file are located. The default value is configured to match the default output directory of
    BuildEngineCore.ps1.
#>

[CmdletBinding()]
param(
    [Alias("p")]
    [string]$projectDirectory,
    [Parameter(Mandatory = $true)]
    [Alias("o")]
    [string]$outputDirectory,
    [string]$dotnetCommand = "dotnet",
    [string]$buildConfiguration = "Release",
    [switch]$noDeployEngineCore,
    [string]$engineCoreDirectory
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = "Stop"

$sourceDirectory = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

if ($projectDirectory -eq "") {
    $projectDirectory = Get-Location
}

# dupe with `BuildEngineCore.ps1`
if ($engineCoreDirectory -eq "") {
    $engineCoreDirectory = Join-Path $sourceDirectory "Derived/EngineCore"
}

Write-Output "============= BuildMacOSAppBundle starting ================="
Write-Output ""

function ConvertFrom-Plist-Node {
    param( [System.Xml.XmlElement] $node )
    switch ($node.Name) {
        "dict" {
            $hash = @{}
            $node.GetElementsByTagName("key") | ForEach-Object {
                $value = ConvertFrom-Plist-Node $_.NextSibling
                $hash.Add($_.InnerText, $value)
            }
            $hash
        }
        "string" {
            $node.InnerText
        }
        "array" {
            $node.ChildNodes |
                Where-Object { $_ -is [System.Xml.XmlElement] } |
                ForEach-Object { ConvertFrom-Plist-Node $_ }
        }
        Default {
            throw "Unknown Plist element '$($node.Name)'."
        }
    }
}

# Use the value of `CFBundleName` as the bundle name
$infoPath = Join-Path $projectDirectory "MacOSBundle" "Info.plist"
[xml]$infoXml = Get-Content $infoPath
[hashtable]$info = ConvertFrom-Plist-Node $infoXml.plist[1].FirstChild
[string]$bundleName = $info.CFBundleName

Write-Verbose "Using the value of 'CFBundleName' ('$bundleName') as the bundle name."

$bundlePath = Join-Path $outputDirectory "$bundleName.app"
Write-Verbose "Bundle path = $bundlePath"

# Publish .NET app
Set-Location $projectDirectory
Write-Output "Publishing the .NET project."

&$dotnetCommand publish -c $buildConfiguration -r osx-x64 `
    -o (Join-Path $bundlePath "Contents" "MacOS")

# Copy the engine core dylibs
if (!$noDeployEngineCore) {
    Write-Output "Copying the engine core files."
    (
        (Get-ChildItem -Path $engineCoreDirectory -Filter "*.dylib") +
        (Get-ChildItem -Path $engineCoreDirectory -Filter "NgsLoaderConfig.xml")
    ) | ForEach-Object {
        Write-Verbose "  $($_.Name)"
        Copy-Item $_.FullName (Join-Path $bundlePath "Contents" "MacOS") -Recurse
    }
}
else {
    Write-Output "Not copying the engine core files because 'noDeployEngineCore' was specified."
}

# Copy bundle resources
Write-Output "Copying the bundle resources."
Get-ChildItem (Join-Path $projectDirectory "MacOSBundle") | ForEach-Object {
    Write-Verbose "  $($_.Name)"
    Copy-Item $_.FullName (Join-Path $bundlePath "Contents") -Recurse
}

Write-Output ""
Write-Output "=============== BuildMacOSAppBundle done ==================="
