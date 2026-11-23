; Offer Ruf as a default-app candidate without replacing the shared tel protocol.
!define RUF_TEL_PROGID "${BUNDLEID}.Tel"
!define RUF_CAPABILITIES "Software\${BUNDLEID}\Capabilities"

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\${RUF_TEL_PROGID}" "" "Ruf telephone link"
  WriteRegStr SHCTX "Software\Classes\${RUF_TEL_PROGID}" "URL Protocol" ""
  WriteRegStr SHCTX "Software\Classes\${RUF_TEL_PROGID}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\${RUF_TEL_PROGID}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --open-tel $\"%1$\""
  WriteRegStr SHCTX "${RUF_CAPABILITIES}" "ApplicationName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${RUF_CAPABILITIES}" "ApplicationDescription" "Ruf SIP softphone"
  WriteRegStr SHCTX "${RUF_CAPABILITIES}\UrlAssociations" "tel" "${RUF_TEL_PROGID}"
  WriteRegStr SHCTX "Software\RegisteredApplications" "${PRODUCTNAME}" "${RUF_CAPABILITIES}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

; Match the installed-release COM activator in baresip-adapter/notifications/windows.rs.
; Notifications are per-user; never touch another application's registrations.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue SHCTX "Software\RegisteredApplications" "${PRODUCTNAME}"
  DeleteRegKey SHCTX "${RUF_CAPABILITIES}"
  DeleteRegKey SHCTX "Software\Classes\${RUF_TEL_PROGID}"
  DeleteRegKey /ifempty SHCTX "Software\${BUNDLEID}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  DeleteRegKey HKCU "Software\Classes\CLSID\{A14373DB-08E7-45EA-9E24-26CB70BFA7B1}"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\${BUNDLEID}"
!macroend
