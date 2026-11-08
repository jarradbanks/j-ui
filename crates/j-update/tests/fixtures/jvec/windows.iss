; Native per-user install. User artwork, preferences and recovery are never installer payloads.
#ifndef Version
  #error Version is required
#endif
#ifndef Payload
  #error Payload is required
#endif
#ifndef Output
  #error Output is required
#endif
#ifndef Identity
  #define Identity "VectorizeDesktop"
#endif
[Setup]
AppId={#Identity}
AppName=jvec
AppVersion={#Version}
DefaultDirName={localappdata}\Programs\jvec
DefaultGroupName=jvec
UsePreviousGroup=no
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19045
OutputDir={#Output}
OutputBaseFilename=vectorize-{#Version}-windows-x64-setup
SetupIconFile=icon.ico
Compression=lzma2
SolidCompression=yes
CloseApplications=no
RestartApplications=no
AppMutex=Local\{#Identity}Installed
DisableProgramGroupPage=yes
#ifdef Fixture
Uninstallable=no
CreateUninstallRegKey=no
UsePreviousAppDir=no
#else
UsePreviousAppDir=yes
UninstallDisplayIcon={app}\jvec.exe
#endif
[Files]
Source: "{#Payload}\*"; DestDir: "{app}"; Excludes: "vectorize-portable,PORTABLE-README.txt"; Flags: ignoreversion recursesubdirs createallsubdirs
; Older signed update helpers restart this filename after upgrading.
Source: "{#Payload}\jvec.exe"; DestDir: "{app}"; DestName: "vectorize-desktop.exe"; Flags: ignoreversion
Source: "windows-installer"; DestDir: "{app}"; DestName: "vectorize-distribution"; Flags: ignoreversion
#ifndef Fixture
[Icons]
Name: "{group}\jvec"; Filename: "{app}\jvec.exe"
[InstallDelete]
Type: files; Name: "{userprograms}\Vectorize\Vectorize.lnk"
#endif
[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
var Root: String; Marker: AnsiString;
begin
  Root := ExpandConstant('{app}');
  Result := '';
  if FileExists(Root + '\vectorize-portable') or
     ((FileExists(Root + '\jvec.exe') or FileExists(Root + '\vectorize-desktop.exe')) and
      (not LoadStringFromFile(Root + '\vectorize-distribution', Marker) or
       (Trim(String(Marker)) <> 'windows-installer'))) then
    Result := 'Refusing portable or unmarked program directory. Choose a new installation folder.';
end;
