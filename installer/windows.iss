; Installeur Windows — Happy Accident Generator (BRNB)
; Compilé automatiquement par GitHub Actions avec Inno Setup 6 :
;   ISCC.exe /DAppVersion=0.1.0 installer\windows.iss

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{B27B2026-0A0C-4D1E-9F00-0000000000A1}
AppName=Happy Accident Generator
AppVersion={#AppVersion}
AppVerName=Happy Accident Generator {#AppVersion}
AppPublisher=BRNB
DefaultDirName={commoncf64}\VST3
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
OutputDir=..\target\installer
OutputBaseFilename=HappyAccidentGenerator-{#AppVersion}-Windows-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
UninstallDisplayName=Happy Accident Generator (BRNB)
WizardStyle=modern
LicenseFile=..\LICENSE

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Types]
Name: "full"; Description: "{cm:TypeFull}"
Name: "custom"; Description: "{cm:TypeCustom}"; Flags: iscustom

[Components]
Name: "vst3"; Description: "VST3 (Ableton Live 10.1+, FL Studio, Cubase…)"; Types: full custom; Flags: fixed
Name: "clap"; Description: "CLAP (Bitwig, Reaper…)"; Types: full

[Files]
Source: "..\target\bundled\Happy Accident Generator.vst3\*"; DestDir: "{commoncf64}\VST3\Happy Accident Generator.vst3"; Components: vst3; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\target\bundled\Happy Accident Generator.clap"; DestDir: "{commoncf64}\CLAP"; Components: clap; Flags: ignoreversion

[CustomMessages]
fr.TypeFull=Installation complète
fr.TypeCustom=Installation personnalisée
en.TypeFull=Full installation
en.TypeCustom=Custom installation

[Messages]
fr.FinishedLabel=Happy Accident Generator est installé.%n%nDans Ableton : Préférences > Plug-ins > activer « Use VST3 Plug-in System Folders » > Rescan. Le plugin apparaît dans Plug-ins > BRNB.
en.FinishedLabel=Happy Accident Generator is installed.%n%nIn Ableton: Preferences > Plug-ins > enable "Use VST3 Plug-in System Folders" > Rescan. The plugin shows up under Plug-ins > BRNB.
