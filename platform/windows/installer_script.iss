[Setup]
AppName=Gem Player
AppVersion={#AppVersion}
DefaultDirName={autopf}\Gem Player
PrivilegesRequired=admin

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start menu shortcut
Name: "{group}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"

; Optional desktop shortcut
Name: "{commondesktop}\Gem Player"; Filename: "{app}\gem-player.exe"; IconFilename: "{app}\gem-player.exe"; Tasks: desktopicon