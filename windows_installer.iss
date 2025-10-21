; Gem Player Inno Setup installer script

[Setup]
AppName=Gem Player
AppVersion={#AppVersion}
DefaultDirName={commonpf}\Gem Player
; Current directory.
OutputDir=.
OutputBaseFilename=GemPlayerInstaller

[Files]
Source: "{{exe_path}}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"

; Desktop shortcut
Name: "{commondesktop}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"
