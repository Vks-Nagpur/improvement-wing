; LedgerCraft Windows installer (NSIS 3).
; Per-user install: no administrator rights needed. The user's work lives in
; Documents\LedgerCraft Data and is never touched by install or uninstall.
;
; Build (from the ledgercraft folder, after `cargo build --release -p lc-app -p lc-cli`
; and copying files into dist\):  makensis /DVERSION=0.1.0 installer\ledgercraft.nsi

Unicode true
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef DIST
  !define DIST "..\dist"
!endif

!include "MUI2.nsh"

Name "LedgerCraft ${VERSION}"
OutFile "${DIST}\LedgerCraft-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\LedgerCraft"
InstallDirRegKey HKCU "Software\LedgerCraft" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "LedgerCraft"
VIAddVersionKey "FileDescription" "LedgerCraft setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Free to use"

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\LedgerCraft.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open LedgerCraft now"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\docs"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Show the user guides (English, Hindi, Marathi)"
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

!define UNINST "Software\Microsoft\Windows\CurrentVersion\Uninstall\LedgerCraft"

Section "LedgerCraft" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${DIST}\LedgerCraft.exe"
  File "${DIST}\ledgercraft-cli.exe"
  File "${DIST}\README.md"
  File "${DIST}\LICENSE-*.txt"
  SetOutPath "$INSTDIR\docs"
  File /r "${DIST}\docs\*.*"
  SetOutPath "$INSTDIR\samples\practice"
  File /r "${DIST}\practice\*.*"
  SetOutPath "$INSTDIR"

  CreateDirectory "$SMPROGRAMS\LedgerCraft"
  CreateShortcut "$SMPROGRAMS\LedgerCraft\LedgerCraft.lnk" "$INSTDIR\LedgerCraft.exe"
  CreateShortcut "$SMPROGRAMS\LedgerCraft\User guides.lnk" "$INSTDIR\docs"
  CreateShortcut "$SMPROGRAMS\LedgerCraft\Practice books.lnk" "$INSTDIR\samples\practice"
  CreateShortcut "$SMPROGRAMS\LedgerCraft\Uninstall LedgerCraft.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\LedgerCraft.lnk" "$INSTDIR\LedgerCraft.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\LedgerCraft" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST}" "DisplayName" "LedgerCraft"
  WriteRegStr HKCU "${UNINST}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST}" "Publisher" "LedgerCraft"
  WriteRegStr HKCU "${UNINST}" "DisplayIcon" "$INSTDIR\LedgerCraft.exe"
  WriteRegStr HKCU "${UNINST}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  ; Program files only. Documents\LedgerCraft Data (the user's work) stays.
  Delete "$INSTDIR\LedgerCraft.exe"
  Delete "$INSTDIR\ledgercraft-cli.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE-*.txt"
  RMDir /r "$INSTDIR\docs"
  RMDir /r "$INSTDIR\samples"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  RMDir /r "$SMPROGRAMS\LedgerCraft"
  Delete "$DESKTOP\LedgerCraft.lnk"
  DeleteRegKey HKCU "${UNINST}"
  DeleteRegKey HKCU "Software\LedgerCraft"
SectionEnd
