$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path "$PSScriptRoot\.."
Set-Location $RootDir

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$appName = $metadata.packages[0].metadata.bundle.name
$appVersion = $metadata.packages[0].version

Write-Host "Building $appName v$appVersion..."

cargo build --release --target x86_64-pc-windows-msvc

$installerDir = "target\release\installer"
New-Item -ItemType Directory -Force $installerDir | Out-Null

$exePath = (Resolve-Path "target\release\gem-player.exe").Path

Write-Host "Building installer..."

iscc `
    "/DAppVersion=$appVersion" `
    "/DExePath=$exePath" `
    "/O$installerDir" `
    "/Fgem_player_${appVersion}_windows_x64_installer" `
    "platform\windows\installer_script.iss"

Write-Host "Windows build complete!"