; Installer RafLand (Inno Setup 6)
[Setup]
AppName=RafLand
AppVersion=0.1.0
AppPublisher=RafLand
DefaultDirName={autopf}\RafLand
DefaultGroupName=RafLand
OutputDir=..\dist
OutputBaseFilename=RafLand-Setup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
UninstallDisplayName=RafLand

[Tasks]
Name: "desktopicon"; Description: "Buat ikon di desktop"; Flags: unchecked
Name: "autostart"; Description: "Jalankan RafLand saat Windows mulai"; Flags: unchecked

[Files]
Source: "..\target\release\rafland.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\RafLand"; Filename: "{app}\rafland.exe"
Name: "{autodesktop}\RafLand"; Filename: "{app}\rafland.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "RafLand"; ValueData: """{app}\rafland.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\rafland.exe"; Description: "Jalankan RafLand"; Flags: nowait postinstall skipifsilent
