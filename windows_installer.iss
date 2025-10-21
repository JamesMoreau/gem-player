; Gem Player Inno Setup installer script

[Setup]
AppName=Gem Player
AppVersion={#AppVersion}
DefaultDirName={commonpf}\Gem Player

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem_player.exe"; IconFilename: "{app}\gem_player.exe"

; Desktop shortcut
Name: "{commondesktop}\Gem Player"; Filename: "{app}\gem_player.exe"; IconFilename: "{app}\gem_player.exe"
