; postbear 安装器脚本（Inno Setup 6）
; 版本号由命令行注入：ISCC.exe /DAppVersion=0.2.0 packaging/postbear.iss

#ifndef AppVersion
#define AppVersion "0.0.0"
#endif

[Setup]
AppName=postbear
AppVersion={#AppVersion}
AppPublisher=ykunyao
DefaultDirName={localappdata}\postbear
OutputBaseFilename=postbear-setup-{#AppVersion}
OutputDir=output
Compression=lzma2
PrivilegesRequired=lowest
UninstallDisplayIcon={app}\postbear.exe
SetupIconFile=..\assets\postbear.ico

[Languages]
Name: "chinese"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式(&D)"; GroupDescription: "附加选项："
Name: "autostart"; Description: "开机自动启动 Bear"; GroupDescription: "附加选项："; Flags: unchecked

[Files]
Source: "..\target\release\postbear.exe"; DestDir: "{app}"
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\postbear"; Filename: "{app}\postbear.exe"
Name: "{autodesktop}\postbear"; Filename: "{app}\postbear.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "postbear"; ValueData: """{app}\postbear.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\postbear.exe"; Description: "立即运行 postbear"; Flags: nowait postinstall skipifsilent
