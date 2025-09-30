; Gem Player Inno Setup installer script

[Setup]
AppName=Gem Player
AppVersion=0.2.0
DefaultDirName={commonpf}\Gem Player
OutputDir=target\x86_64-pc-windows-gnu\debug
OutputBaseFilename=GemPlayerInstaller

[Files]
Source: "target\x86_64-pc-windows-gnu\debug\gem-player.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem-player.exe"

; Desktop shortcut
Name: "{commondesktop}\Gem Player"; Filename: "{app}\gem-player.exe"