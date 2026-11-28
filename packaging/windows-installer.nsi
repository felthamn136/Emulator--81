Unicode true
!include "MUI2.nsh"

!define APP_NAME "Membrane81"
!define APP_VERSION "0.1.0"
!define APP_EXE "membrane81.exe"
!define PROJECT_ROOT "${__FILEDIR__}/.."
!ifndef WINDOWS_BINARY
!define WINDOWS_BINARY "target/x86_64-pc-windows-gnu/release/${APP_EXE}"
!endif
!ifndef INSTALLER_OUTPUT
!define INSTALLER_OUTPUT "${PROJECT_ROOT}/dist/Membrane81-Setup.exe"
!endif

!cd "${PROJECT_ROOT}"

Name "${APP_NAME}"
OutFile "${INSTALLER_OUTPUT}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"
RequestExecutionLevel admin

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "${WINDOWS_BINARY}"
  File "packaging/membrane81.ico"

  SetOutPath "$INSTDIR\rom"
  File "rom/zx81.rom"

  SetOutPath "$INSTDIR\assets"
  File /r "assets/*"

  SetOutPath "$INSTDIR\Games Library"
  File /r "Games Library/*"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\membrane81.ico"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayIcon" "$INSTDIR\membrane81.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
  RMDir /r "$INSTDIR"
SectionEnd

Function un.onInit
  ReadRegStr $INSTDIR HKLM "Software\${APP_NAME}" "InstallDir"
FunctionEnd
