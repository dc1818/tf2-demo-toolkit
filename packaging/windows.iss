[Setup]
AppId={{6783ED83-0397-4602-8C17-18F02DCE18DD}
AppName=TF2 Demo Toolkit
AppVersion=34.0.0
AppPublisher=dc1818
AppPublisherURL=https://github.com/dc1818/tf2-demo-toolkit
DefaultDirName={localappdata}\Programs\TF2 Demo Toolkit
DefaultGroupName=TF2 Demo Toolkit
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\release-assets
OutputBaseFilename=TF2-Demo-Toolkit-Windows-x64-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\TF2_Demo_Toolkit.exe
[Files]
Source: "..\dist\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{group}\TF2 Demo Toolkit"; Filename: "{app}\TF2_Demo_Toolkit.exe"
[Run]
Filename: "{app}\TF2_Demo_Toolkit.exe"; Description: "Open TF2 Demo Toolkit"; Flags: nowait postinstall skipifsilent
