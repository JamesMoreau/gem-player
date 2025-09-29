; Gem Player Inno Setup installer script

[Setup]
AppName=Gem Player
AppVersion=0.2.0
DefaultDirName={commonpf}\Gem Player
DefaultGroupName=Gem Player
OutputBaseFilename=GemPlayerInstaller
Compression=lzma
SolidCompression=yes

[Files]
Source: "gem-player.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem-player.exe"

; Desktop shortcut
Name: "{commondesktop}\Gem Player"; Filename: "{app}\gem-player.exe"
