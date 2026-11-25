param(
    [switch]$Debug,
    [switch]$Unregister
)

$ErrorActionPreference = "Stop"

$root = Split-Path $PSScriptRoot -Parent
$profile = if ($Debug) { "debug" } else { "release" }
$exe = Join-Path $root "target\$profile\mdviewer-by-ali.exe"

if (-not (Test-Path $exe)) {
    throw "Executable not found: $exe. Run .\scripts\dev.ps1 -BuildOnly$(if (-not $Debug) { ' -Release' }) first."
}

$flag = if ($Unregister) { "--unregister" } else { "--register" }
& $exe $flag
if ($LASTEXITCODE -ne 0) {
    throw "Registration command failed with exit code $LASTEXITCODE"
}
Write-Host "Successfully executed $flag on $exe"
