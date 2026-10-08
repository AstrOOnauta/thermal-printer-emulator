; NSIS installer hooks (bundle.windows.nsis.installerHooks in tauri.conf.json).
;
; Windows Firewall: the emulator receives print jobs from POS terminals on the network.
; The rule is for the program, not a port, so it keeps working when Settings picks another
; port, and only for private and domain networks: never public ones (a café's Wi-Fi). Added
; on install (replaced on an update), removed on uninstall but kept through an update.
; `netsh` needs admin, which the per-machine installer has.

!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="${PRODUCTNAME}"'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="${PRODUCTNAME}" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" enable=yes profile=private,domain'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    nsExec::Exec 'netsh advfirewall firewall delete rule name="${PRODUCTNAME}"'
    Pop $0
  ${EndIf}
!macroend
