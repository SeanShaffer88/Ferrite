; Forgepad MD Portable - PortableApps.com Format Installer
; This script creates a .paf.exe self-extracting installer that follows
; the PortableApps.com Format specification.
;
; Build:
;   makensis /DAPPVERSION=0.3.1 installer.nsi
;
; If APPVERSION is not passed, defaults to "0.0.0".

Unicode true

!include "MUI2.nsh"
!include "FileFunc.nsh"

; --- Application metadata ---
!define APPNAME "Forgepad MD Portable"
!define APPID "ForgepadPortable"
!ifndef APPVERSION
  !define APPVERSION "0.0.0"
!endif
!define APPPUBLISHER "OlaProeis"
!define APPHOMEPAGE "https://github.com/OlaProeis/Ferrite"

; --- Installer settings ---
Name "${APPNAME} ${APPVERSION}"
OutFile "${APPID}_${APPVERSION}_English.paf.exe"
InstallDir "\${APPID}"
ShowInstDetails nevershow

; --- Compression ---
SetCompressor /SOLID lzma
SetCompressorDictSize 32

; --- MUI settings ---
!define MUI_ICON "ForgepadPortable\App\AppInfo\appicon.ico"
!define MUI_ABORTWARNING

; --- Directory page ---
!define MUI_DIRECTORYPAGE_TEXT_TOP "Setup will install ${APPNAME} ${APPVERSION} in the following folder. To install in a different folder, click Browse and select another folder. Click Install to start the installation."
!define MUI_DIRECTORYPAGE_TEXT_DESTINATION "Destination Folder"

; --- Pages ---
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

; --- Language ---
!insertmacro MUI_LANGUAGE "English"

; --- Install section ---
Section "Install"
    SetOutPath "$INSTDIR"

    ; Copy the entire ForgepadPortable directory structure
    File "ForgepadPortable\ForgepadPortable.exe"
    File "ForgepadPortable\help.html"

    SetOutPath "$INSTDIR\App\AppInfo"
    File "ForgepadPortable\App\AppInfo\appinfo.ini"
    File "ForgepadPortable\App\AppInfo\appicon.ico"
    File "ForgepadPortable\App\AppInfo\appicon_16.png"
    File "ForgepadPortable\App\AppInfo\appicon_32.png"
    File "ForgepadPortable\App\AppInfo\appicon_128.png"

    SetOutPath "$INSTDIR\App\AppInfo\Launcher"
    File "ForgepadPortable\App\AppInfo\Launcher\ForgepadPortable.ini"

    SetOutPath "$INSTDIR\App\Forgepad"
    File "ForgepadPortable\App\Forgepad\forgepad.exe"

    SetOutPath "$INSTDIR\App\DefaultData\settings"

    SetOutPath "$INSTDIR\Data\settings"

    SetOutPath "$INSTDIR\Other\Source"
    File "ForgepadPortable\Other\Source\LICENSE"

    SetOutPath "$INSTDIR\Other\Help"
    File "ForgepadPortable\Other\Help\help.html"

    CreateDirectory "$INSTDIR\Other\Help\Images"
SectionEnd
