; OSMOzzz Windows Installer
; Built with NSIS (Nullsoft Scriptable Install System)
; Mirrors the macOS .pkg behaviour:
;   - installs binary + DLL to Program Files\OSMOzzz\
;   - copies ONNX models to %USERPROFILE%\.osmozzz\models\
;   - sets ORT_DYLIB_PATH system env var
;   - registers auto-start at login (HKCU Run key)
;   - creates Start Menu shortcuts
;   - registers in Add/Remove Programs with a clean uninstaller

Unicode True

!define APP_NAME      "OSMOzzz"
!define APP_EXE       "osmozzz.exe"
!define ORG_NAME      "OSMOzzz"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\OSMOzzz"
!define RUN_KEY       "Software\Microsoft\Windows\CurrentVersion\Run"

; VERSION and DIST_DIR are injected at build time:
;   makensis /DVERSION=1.2.3 /DDIST_DIR=C:\...\dist-windows installer.nsi
!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef DIST_DIR
  !define DIST_DIR "dist-windows"
!endif

Name            "${APP_NAME} ${VERSION}"
OutFile         "osmozzz-setup.exe"
InstallDir      "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor   /SOLID lzma
ShowInstDetails show

; ── UI ────────────────────────────────────────────────────────────────────────
!include "MUI2.nsh"
!include "x64.nsh"

!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN            "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_PARAMETERS "daemon"
!define MUI_FINISHPAGE_RUN_TEXT       "Start OSMOzzz now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ── Install ───────────────────────────────────────────────────────────────────
Section "OSMOzzz" SecMain

    ; 1. Binary + DLL → Program Files\OSMOzzz\
    SetOutPath "$INSTDIR"
    File "${DIST_DIR}\osmozzz.exe"
    File "${DIST_DIR}\onnxruntime.dll"

    ; 2. ONNX models → %USERPROFILE%\.osmozzz\models\  (mirrors macOS postinstall)
    CreateDirectory "$PROFILE\.osmozzz\models"
    SetOutPath "$PROFILE\.osmozzz\models"
    File "${DIST_DIR}\models\all-MiniLM-L6-v2.onnx"
    File "${DIST_DIR}\models\tokenizer.json"

    ; 3. ORT_DYLIB_PATH — system-wide, broadcast to running processes
    WriteRegExpandStr HKLM \
        "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
        "ORT_DYLIB_PATH" "$INSTDIR\onnxruntime.dll"
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000

    ; 4. Auto-start daemon at Windows login (current user, no UAC prompt)
    WriteRegStr HKCU "${RUN_KEY}" "${APP_NAME}" '"$INSTDIR\${APP_EXE}" daemon'

    ; 5. Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" \
        "$INSTDIR\${APP_EXE}" "daemon"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Dashboard.lnk" \
        "http://localhost:7878"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" \
        "$INSTDIR\uninstall.exe"

    ; 6. Add/Remove Programs entry
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "DisplayName"     "${APP_NAME}"
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "UninstallString"  '"$INSTDIR\uninstall.exe"'
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "InstallLocation"  "$INSTDIR"
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "DisplayVersion"   "${VERSION}"
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "Publisher"        "${ORG_NAME}"
    WriteRegStr   HKLM "${UNINSTALL_KEY}" "URLInfoAbout"     "https://osmozzz.dev"
    WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify"         1
    WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair"         1

SectionEnd

; ── Uninstall ─────────────────────────────────────────────────────────────────
; Note: ~/.osmozzz/ vault and config are intentionally preserved (user data)
Section "Uninstall"

    ExecWait 'taskkill /F /IM ${APP_EXE}' $0

    Delete "$INSTDIR\${APP_EXE}"
    Delete "$INSTDIR\onnxruntime.dll"
    Delete "$INSTDIR\uninstall.exe"
    RMDir  "$INSTDIR"

    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Dashboard.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
    RMDir  "$SMPROGRAMS\${APP_NAME}"

    DeleteRegValue HKCU "${RUN_KEY}" "${APP_NAME}"
    DeleteRegValue HKLM \
        "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" \
        "ORT_DYLIB_PATH"
    DeleteRegKey HKLM "${UNINSTALL_KEY}"

SectionEnd
