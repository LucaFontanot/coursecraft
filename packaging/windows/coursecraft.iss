#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef OutputName
  #define OutputName "CourseCraft-setup"
#endif
#ifndef RepoRoot
  #define RepoRoot "..\.."
#endif
#ifndef OutputDir
  #define OutputDir RepoRoot + "\dist"
#endif
#ifndef SourceExe
  #define SourceExe RepoRoot + "\target\release\coursecraft.exe"
#endif

[Setup]
AppId={{7B6E3A4C-5F1A-4D8E-9C2B-8A1F3E5D7C90}
AppName=CourseCraft
AppVersion={#AppVersion}
AppVerName=CourseCraft {#AppVersion}
AppPublisher=CourseCraft
DefaultDirName={localappdata}\Programs\CourseCraft
DefaultGroupName=CourseCraft
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
SetupIconFile={#RepoRoot}\apps\coursecraft\icons\coursecraft.ico
UninstallDisplayIcon={app}\coursecraft.exe
UninstallDisplayName=CourseCraft
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\ATTRIBUTION.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\CourseCraft"; Filename: "{app}\coursecraft.exe"; IconFilename: "{app}\coursecraft.exe"
Name: "{autodesktop}\CourseCraft"; Filename: "{app}\coursecraft.exe"; IconFilename: "{app}\coursecraft.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\coursecraft.exe"; Description: "{cm:LaunchProgram,CourseCraft}"; Flags: nowait postinstall skipifsilent
