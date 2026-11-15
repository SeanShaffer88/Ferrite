; Forgepad — Windows Inno Setup installer (optional alternative to MSI)
;
; Builds a per-machine installer from the release binary with Start Menu shortcut,
; optional file associations (OpenWithProgids, matching wix/main.wxs), Explorer
; context menu entries, and optional PATH integration.
;
; Manual build (from repo root, after cargo build --release):
;   powershell -File installer\build.ps1
;   — or —
;   "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DMyAppVersion=0.3.1 installer\forgepad.iss
;
; Output: installer\Output\forgepad-windows-x64-setup.exe

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

#define MyAppName "Forgepad"
#define MyAppPublisher "SeanShaffer88"
#define MyAppURL "https://github.com/SeanShaffer88/Forgepad"
#define MyAppExeName "forgepad.exe"
#define MyAppId "{{BA764367-9962-4541-86D1-40FA88C15599}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf64}\Forgepad
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=Output
OutputBaseFilename=forgepad-windows-x64-setup
SetupIconFile=..\assets\icons\windows\app.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
VersionInfoVersion={#MyAppVersion}.0
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} Setup
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked
; Tasks are checked by default. Inno Setup has no "checked" flag.
Name: "assocmd"; Description: "Markdown (.md, .markdown)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "assoctxt"; Description: "Plain text (.txt)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "assocjson"; Description: "JSON (.json)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "assocyaml"; Description: "YAML (.yaml, .yml)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "assoctoml"; Description: "TOML (.toml)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "assoccsv"; Description: "CSV (.csv, .tsv)"; GroupDescription: "File associations (Open With / Default Apps):"
Name: "contextmenufiles"; Description: "&Open with Forgepad (any file)"; GroupDescription: "Explorer context menu:"; Flags: unchecked
Name: "contextmenufolders"; Description: "Open &Folder with Forgepad (directories)"; GroupDescription: "Explorer context menu:"; Flags: unchecked
Name: "addtopath"; Description: "Add Forgepad to the system &PATH"; GroupDescription: "Other:"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
; --- Markdown ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.md"; ValueType: string; ValueData: "Markdown Document"; Flags: uninsdeletekey; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\Forgepad.md\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\Forgepad.md\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.md"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\Classes\.markdown\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.md"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocmd

; --- Plain text ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.txt"; ValueType: string; ValueData: "Text Document"; Flags: uninsdeletekey; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\Forgepad.txt\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\Forgepad.txt\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoctxt
Root: HKLM; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.txt"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoctxt

; --- JSON ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.json"; ValueType: string; ValueData: "JSON Document"; Flags: uninsdeletekey; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\Forgepad.json\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\Forgepad.json\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocjson
Root: HKLM; Subkey: "Software\Classes\.json\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.json"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocjson

; --- YAML ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.yaml"; ValueType: string; ValueData: "YAML Document"; Flags: uninsdeletekey; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\Forgepad.yaml\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\Forgepad.yaml\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\.yaml\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.yaml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\Classes\.yml\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.yaml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assocyaml

; --- TOML ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.toml"; ValueType: string; ValueData: "TOML Document"; Flags: uninsdeletekey; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\Forgepad.toml\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\Forgepad.toml\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoctoml
Root: HKLM; Subkey: "Software\Classes\.toml\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.toml"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoctoml

; --- CSV ---
Root: HKLM; Subkey: "Software\Classes\Forgepad.csv"; ValueType: string; ValueData: "CSV Document"; Flags: uninsdeletekey; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\Forgepad.csv\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\Forgepad.csv\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\.csv\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.csv"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\Classes\.tsv\OpenWithProgids"; ValueType: string; ValueName: "Forgepad.csv"; ValueData: ""; Flags: uninsdeletevalue; Tasks: assoccsv

; --- Windows Default Apps registration (per selected extension) ---
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities"; ValueName: "ApplicationDescription"; ValueType: string; ValueData: "A fast, lightweight text editor for Markdown, JSON, and more"; Flags: uninsdeletekey; Check: AnyAssocTaskSelected
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities"; ValueName: "ApplicationName"; ValueType: string; ValueData: "{#MyAppName}"; Check: AnyAssocTaskSelected
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".md"; ValueType: string; ValueData: "Forgepad.md"; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".markdown"; ValueType: string; ValueData: "Forgepad.md"; Flags: uninsdeletevalue; Tasks: assocmd
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".txt"; ValueType: string; ValueData: "Forgepad.txt"; Flags: uninsdeletevalue; Tasks: assoctxt
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".json"; ValueType: string; ValueData: "Forgepad.json"; Flags: uninsdeletevalue; Tasks: assocjson
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".yaml"; ValueType: string; ValueData: "Forgepad.yaml"; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".yml"; ValueType: string; ValueData: "Forgepad.yaml"; Flags: uninsdeletevalue; Tasks: assocyaml
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".toml"; ValueType: string; ValueData: "Forgepad.toml"; Flags: uninsdeletevalue; Tasks: assoctoml
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".csv"; ValueType: string; ValueData: "Forgepad.csv"; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\SeanShaffer88\Forgepad\Capabilities\FileAssociations"; ValueName: ".tsv"; ValueType: string; ValueData: "Forgepad.csv"; Flags: uninsdeletevalue; Tasks: assoccsv
Root: HKLM; Subkey: "Software\RegisteredApplications"; ValueName: "Forgepad"; ValueType: string; ValueData: "Software\SeanShaffer88\Forgepad\Capabilities"; Flags: uninsdeletevalue; Check: AnyAssocTaskSelected

; --- Explorer context menu ---
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithForgepad"; ValueType: string; ValueData: "Open with Forgepad"; Flags: uninsdeletekey; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithForgepad"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\*\shell\OpenWithForgepad\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: contextmenufiles
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithForgepad"; ValueType: string; ValueData: "Open Folder with Forgepad"; Flags: uninsdeletekey; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithForgepad"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\shell\OpenWithForgepad\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithForgepad"; ValueType: string; ValueData: "Open Folder with Forgepad"; Flags: uninsdeletekey; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithForgepad"; ValueName: "Icon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: contextmenufolders
Root: HKLM; Subkey: "Software\Classes\Directory\Background\shell\OpenWithForgepad\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: contextmenufolders

; --- Install marker (Start Menu shortcut key path analogue) ---
Root: HKCU; Subkey: "Software\SeanShaffer88\Forgepad"; ValueName: "installed"; ValueType: dword; ValueData: "1"; Flags: uninsdeletekey

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent unchecked

[Code]
function AnyAssocTaskSelected: Boolean;
begin
  Result :=
    IsTaskSelected('assocmd') or
    IsTaskSelected('assoctxt') or
    IsTaskSelected('assocjson') or
    IsTaskSelected('assocyaml') or
    IsTaskSelected('assoctoml') or
    IsTaskSelected('assoccsv');
end;

function AddToPath(Param: string): string;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Result := Param
  else
    Result := OrigPath + ';' + Param;
end;

function RemoveFromPath(Param: string): string;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Exit;

  if Pos(';' + Param + ';', ';' + OrigPath + ';') > 0 then
    StringChangeEx(OrigPath, ';' + Param, '', True)
  else if Pos(Param + ';', OrigPath) = 1 then
    StringChangeEx(OrigPath, Param + ';', '', True)
  else if Copy(OrigPath, Length(OrigPath) - Length(Param), Length(Param) + 1) = ';' + Param then
    StringChangeEx(OrigPath, ';' + Param, '', False);

  Result := OrigPath;
end;

function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(
    HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
    'Path',
    OrigPath)
  then
    Result := True
  else
    Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if IsTaskSelected('addtopath') and NeedsAddPath(ExpandConstant('{app}')) then
    begin
      RegWriteExpandStringValue(
        HKEY_LOCAL_MACHINE,
        'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
        'Path',
        AddToPath(ExpandConstant('{app}')));
      RegWriteDWordValue(
        HKEY_LOCAL_MACHINE,
        'Software\SeanShaffer88\Forgepad',
        'PathAdded',
        1);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppPath: string;
  PathAdded: Cardinal;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    if RegQueryDWordValue(
      HKEY_LOCAL_MACHINE,
      'Software\SeanShaffer88\Forgepad',
      'PathAdded',
      PathAdded) and (PathAdded = 1) then
    begin
      AppPath := ExpandConstant('{app}');
      if RegValueExists(
        HKEY_LOCAL_MACHINE,
        'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
        'Path') then
      begin
        RegWriteExpandStringValue(
          HKEY_LOCAL_MACHINE,
          'SYSTEM\CurrentControlSet\Control\Session Manager\Environment',
          'Path',
          RemoveFromPath(AppPath));
      end;
      RegDeleteValue(HKEY_LOCAL_MACHINE, 'Software\SeanShaffer88\Forgepad', 'PathAdded');
    end;
  end;
end;
