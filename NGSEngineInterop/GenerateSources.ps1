#!/usr/bin/env powershell

# Source/tool paths
$NGSEngineInteropPath = Split-Path -Parent $PSCommandPath
$NGSEnginePath = Split-Path -Parent $NGSEngineInteropPath |
    % { Join-Path $_ "NGSEngine" }
$NGSCorePath = Split-Path -Parent $NGSEngineInteropPath |
    % { Join-Path $_ "NGSCore" }
$CSInteropToolPath = Join-Path $NGSCorePath "Xpcom/idl-parser/xpidl/csinterop.py"

$CSInteropIncludePaths = `
    ( Join-Path $NGSCorePath "Xpcom/base" ),
    ( Join-Path $NGSCorePath "Utils" ),
    ( $NGSEnginePath )
$CSInteropConfigPath = Join-Path $NGSEngineInteropPath "CSInteropConfig.json"
$CSInteropFlags = $CSInteropIncludePaths | % { "-I" + $_ }
$CSInteropFlags += "-c" + $CSInteropConfigPath
$CSInteropFlags += "--cachedir"
$CSInteropFlags += Join-Path $NGSEngineInteropPath "Generated/xpidlcache"

# Output path
$GeneratedSourcesRoot = Join-Path $NGSEngineInteropPath "Generated"

$NGSEngineCMakeListsPath = Join-Path $NGSEnginePath "CMakeLists.txt"
$IDLFilesString = Get-Content $NGSEngineCMakeListsPath -Raw |
    Select-String -Pattern "(?sm)set\(IDL_FILES\b(.*?)\)" -AllMatches |
    % { $_.Matches.Groups[1].Value }
$IDLFiles = $IDLFilesString |
    Select-String -Pattern "\b([a-zA-Z0-9_\/]+\.idl)" -AllMatches |
    % { $_.Matches } |
    % { $_.Value }

ForEach ($IDLFile in $IDLFiles) {
    $IDLPath = Join-Path $NGSEnginePath $IDLFile
    $CSPath = [IO.Path]::GetFileNameWithoutExtension($IDLPath) |
        % { Join-Path $GeneratedSourcesRoot ($_ + ".cs") }

    Echo ("Processing " + $IDLFile)

    $CommandArgs = $CSInteropFlags
    $CommandArgs += "-o" + $CSPath
    $CommandArgs += $IDLPath

    & $CSInteropToolPath $CommandArgs
}