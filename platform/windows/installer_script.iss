; Gem Player Inno Setup installer script

[Setup]
AppName=Gem Player
AppVersion={#AppVersion}
DefaultDirName={autopf}\Gem Player

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"

; Desktop shortcut
Name: "{userdesktop}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"
