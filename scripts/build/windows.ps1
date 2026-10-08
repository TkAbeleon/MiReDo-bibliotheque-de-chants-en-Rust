$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$AppDir = Join-Path $RootDir "apps\miredo"
$DistDir = Join-Path $RootDir "dist\windows"
$Target = "x86_64-pc-windows-gnu"
$BinName = "miredo.exe"

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

if (-not (Get-Command rustc -ErrorAction SilentlyContinue) -or -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust n'est pas installé. Installation via rustup..."
    Invoke-WebRequest https://sh.rustup.rs -UseBasicParsing | Select-Object -ExpandProperty Content | Out-File "$env:TEMP\rustup-init.sh"
    bash "$env:TEMP\rustup-init.sh" -y
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

rustup target add $Target
cargo build --manifest-path (Join-Path $AppDir "Cargo.toml") --release --target $Target
Copy-Item (Join-Path $AppDir "target\$Target\release\$BinName") (Join-Path $DistDir "miredo-windows-x86_64.exe")

Write-Host "Binaire généré : $(Join-Path $DistDir "miredo-windows-x86_64.exe")"
