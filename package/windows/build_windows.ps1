$ErrorActionPreference = "Stop"

$ScriptDir = $PSScriptRoot
$RootDir = Resolve-Path "$ScriptDir\..\.."
Set-Location $RootDir

$InstallerScript = Join-Path $ScriptDir "installer_script.iss"

if (-not (Get-Command "ISCC.exe" -ErrorAction SilentlyContinue)) {
    throw "Inno Setup Compiler (ISCC.exe) was not found. Install Inno Setup or add ISCC.exe to PATH."
}

$Metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$Package = $Metadata.packages | Where-Object { $_.name -eq "gem-player" } | Select-Object -First 1

if (-not $Package) {
    throw "Could not find the gem-player package in Cargo metadata."
}

$AppName = $Package.metadata.bundle.name
$AppVersion = $Package.version

if (-not $AppName) {
    $AppName = $Package.name
}

Write-Host "Building $AppName v$AppVersion..."

cargo build --release --target x86_64-pc-windows-msvc

$InstallerDir = Join-Path $RootDir "target\release\installer"
New-Item -ItemType Directory -Force $InstallerDir | Out-Null

$ExePath = Join-Path $RootDir "target\x86_64-pc-windows-msvc\release\gem-player.exe"

if (-not (Test-Path $ExePath)) {
    throw "Build executable not found: $ExePath"
}

Write-Host "Building installer..."

& ISCC.exe `
    "/DAppVersion=$AppVersion" `
    "/DExePath=$ExePath" `
    "/O$InstallerDir" `
    "/Fgem_player_${AppVersion}_windows_x64_installer" `
    $InstallerScript

if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE."
}

Write-Host "Windows build complete!"