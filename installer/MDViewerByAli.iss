#define MyAppName "MDViewerByAli"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "Ali"
#define MyAppExeName "MDViewerByAli.exe"

[Setup]
AppId={{D9E7C4B1-8A42-4F6B-9E3A-1C2D3E4F5A6B}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=MDViewerByAli-0.2.0-Setup
SetupIconFile=..\assets\app\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName},0
UninstallDisplayName={#MyAppName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "associate_md"; Description: "Associate .md, .markdown, .mmd, .mermaid files (Open With & File Icons)"; GroupDescription: "File Associations:"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\mdviewer-by-ali.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"; IconIndex: 0
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"; IconIndex: 0; Tasks: desktopicon

[Registry]
; ProgID: Markdown Document (uses embedded icon index 1 = md-doc.ico)
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Markdown"; ValueType: string; ValueName: ""; ValueData: "Markdown Document"; Flags: uninsdeletekey; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Markdown"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "Markdown Document"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Markdown\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"",1"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Markdown\shell\open"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Markdown\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associate_md

; ProgID: Mermaid Diagram (uses embedded icon index 1 = md-doc.ico)
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Mermaid"; ValueType: string; ValueName: ""; ValueData: "Mermaid Diagram"; Flags: uninsdeletekey; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Mermaid"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "Mermaid Diagram"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Mermaid\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"",1"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Mermaid\shell\open"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\MDViewerByAli.Mermaid\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associate_md

; Application Registration (uses app icon index 0 = icon.ico)
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"; Flags: uninsdeletekey; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"",0"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\shell\open"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".md"; ValueData: ""; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".markdown"; ValueData: ""; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".mmd"; ValueData: ""; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".mermaid"; ValueData: ""; Tasks: associate_md

; Extension Defaults & OpenWithProgids (.md, .markdown, .mmd, .mermaid)
Root: HKCU; Subkey: "Software\Classes\.md"; ValueType: string; ValueName: ""; ValueData: "MDViewerByAli.Markdown"; Flags: uninsdeletevalue; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.md"; ValueType: string; ValueName: "PerceivedType"; ValueData: "text"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.md"; ValueType: string; ValueName: "Content Type"; ValueData: "text/markdown"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "MDViewerByAli.Markdown"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_md

Root: HKCU; Subkey: "Software\Classes\.markdown"; ValueType: string; ValueName: ""; ValueData: "MDViewerByAli.Markdown"; Flags: uninsdeletevalue; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.markdown"; ValueType: string; ValueName: "PerceivedType"; ValueData: "text"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.markdown"; ValueType: string; ValueName: "Content Type"; ValueData: "text/markdown"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.markdown\OpenWithProgids"; ValueType: string; ValueName: "MDViewerByAli.Markdown"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_md

Root: HKCU; Subkey: "Software\Classes\.mmd"; ValueType: string; ValueName: ""; ValueData: "MDViewerByAli.Mermaid"; Flags: uninsdeletevalue; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.mmd"; ValueType: string; ValueName: "PerceivedType"; ValueData: "text"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.mmd\OpenWithProgids"; ValueType: string; ValueName: "MDViewerByAli.Mermaid"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_md

Root: HKCU; Subkey: "Software\Classes\.mermaid"; ValueType: string; ValueName: ""; ValueData: "MDViewerByAli.Mermaid"; Flags: uninsdeletevalue; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.mermaid"; ValueType: string; ValueName: "PerceivedType"; ValueData: "text"; Tasks: associate_md
Root: HKCU; Subkey: "Software\Classes\.mermaid\OpenWithProgids"; ValueType: string; ValueName: "MDViewerByAli.Mermaid"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate_md

; Windows Default Apps Capabilities
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"; Flags: uninsdeletekey; Tasks: associate_md
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Offline Markdown & Mermaid Viewer"; Tasks: associate_md
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities\FileAssociations"; ValueType: string; ValueName: ".md"; ValueData: "MDViewerByAli.Markdown"; Tasks: associate_md
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities\FileAssociations"; ValueType: string; ValueName: ".markdown"; ValueData: "MDViewerByAli.Markdown"; Tasks: associate_md
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mmd"; ValueData: "MDViewerByAli.Mermaid"; Tasks: associate_md
Root: HKCU; Subkey: "Software\MDViewerByAli\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mermaid"; ValueData: "MDViewerByAli.Mermaid"; Tasks: associate_md
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: "Software\MDViewerByAli\Capabilities"; Flags: uninsdeletevalue; Tasks: associate_md

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--unregister"; RunOnceId: "UnregisterMDViewerByAli"; Flags: runhidden
