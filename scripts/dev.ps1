param(
    [switch]$BuildOnly,
    [switch]$Release
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue) -and (Test-Path "E:\Rust\.cargo\bin\cargo.exe")) {
    $env:RUSTUP_HOME = "E:\Rust\.rustup"
    $env:CARGO_HOME = "E:\Rust\.cargo"
    $env:PATH = "E:\Rust\.cargo\bin;$env:PATH"
}

if (-not (Get-Command rc.exe -ErrorAction SilentlyContinue)) {
    $sdkBin = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64" -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName "rc.exe") } |
        Sort-Object Name -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if ($sdkBin) {
        $env:PATH = "$sdkBin;$env:PATH"
    }
}

$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    if ($Release) {
        if ($BuildOnly) {
            cargo build --release
        } else {
            cargo run --release
        }
    } else {
        if ($BuildOnly) {
            cargo build
        } else {
            cargo run
        }
    }
} finally {
    Pop-Location
}
