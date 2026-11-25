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

$iscc = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $iscc)) {
    throw "Inno Setup 6 compiler not found at $iscc"
}

$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    cargo test
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test failed with exit code $LASTEXITCODE"
    }
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed with exit code $LASTEXITCODE"
    }
    & $iscc "$root\installer\MDViewerByAli.iss"
    if ($LASTEXITCODE -ne 0) {
        throw "ISCC.exe failed with exit code $LASTEXITCODE"
    }
    $setupExe = Join-Path $root "dist\MDViewerByAli-0.2.0-Setup.exe"
    if (-not (Test-Path $setupExe)) {
        throw "Installer output missing: $setupExe"
    }
    $sizeMB = [math]::Round((Get-Item $setupExe).Length / 1MB, 2)
    Write-Host "INSTALLER READY: $setupExe ($sizeMB MB)"
} finally {
    Pop-Location
}
