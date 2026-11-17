Unicode true

!include "MUI2.nsh"
!include "x64.nsh"

!ifndef APP_VERSION
  !error "APP_VERSION is required"
!endif
!ifndef SOURCE_DIR
  !error "SOURCE_DIR is required"
!endif
!ifndef OUT_FILE
  !error "OUT_FILE is required"
!endif
!ifndef EST_SIZE
  !define EST_SIZE 40000
!endif

!define APP_NAME "SkyLauncher"
!define APP_EXE "SkyLauncher.exe"
!define APP_PUBLISHER "T4skor"
!define APP_REG "Software\SkyLauncher"
!define UNINST_REG "Software\Microsoft\Windows\CurrentVersion\Uninstall\SkyLauncher"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\SkyLauncher"
InstallDirRegKey HKCU "${APP_REG}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64
BrandingText "${APP_NAME} ${APP_VERSION} - ${APP_PUBLISHER}"

!ifdef APP_VERSION_4
  VIProductVersion "${APP_VERSION_4}"
  VIAddVersionKey "ProductName" "${APP_NAME}"
  VIAddVersionKey "ProductVersion" "${APP_VERSION}"
  VIAddVersionKey "FileDescription" "SkyLauncher installer"
  VIAddVersionKey "FileVersion" "${APP_VERSION}"
  VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"
  VIAddVersionKey "LegalCopyright" "MIT License"
!endif

!define MUI_ABORTWARNING
!ifdef ICON_FILE
  !define MUI_ICON "${ICON_FILE}"
  !define MUI_UNICON "${ICON_FILE}"
  !define SHORTCUT_ICON "$INSTDIR\SkyLauncher.ico"
!else
  !define SHORTCUT_ICON "$INSTDIR\${APP_EXE}"
!endif
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Abrir SkyLauncher"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "SkyLauncher requiere Windows de 64 bits."
    Abort
  ${EndIf}
FunctionEnd

Section "SkyLauncher" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File /r "${SOURCE_DIR}/*"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\SkyLauncher"
  CreateShortCut "$SMPROGRAMS\SkyLauncher\SkyLauncher.lnk" "$INSTDIR\${APP_EXE}" "" "${SHORTCUT_ICON}" 0
  CreateShortCut "$SMPROGRAMS\SkyLauncher\Desinstalar SkyLauncher.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortCut "$DESKTOP\SkyLauncher.lnk" "$INSTDIR\${APP_EXE}" "" "${SHORTCUT_ICON}" 0

  WriteRegStr HKCU "${APP_REG}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${APP_REG}" "Version" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINST_REG}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINST_REG}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINST_REG}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKCU "${UNINST_REG}" "DisplayIcon" "${SHORTCUT_ICON}"
  WriteRegStr HKCU "${UNINST_REG}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_REG}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINST_REG}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_REG}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_REG}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINST_REG}" "EstimatedSize" ${EST_SIZE}
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$DESKTOP\SkyLauncher.lnk"
  Delete "$SMPROGRAMS\SkyLauncher\SkyLauncher.lnk"
  Delete "$SMPROGRAMS\SkyLauncher\Desinstalar SkyLauncher.lnk"
  RMDir "$SMPROGRAMS\SkyLauncher"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKCU "${UNINST_REG}"
  DeleteRegKey HKCU "${APP_REG}"
SectionEnd
