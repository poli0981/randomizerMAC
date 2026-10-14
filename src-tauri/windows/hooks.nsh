; RandomMac NSIS installer hooks (see bundle.windows.nsis.installerHooks).

!macro NSIS_HOOK_POSTINSTALL
  ; Remove the elevated logon task that RandomMac 1.x left behind: it pointed at a
  ; user-writable folder, so keeping it would be a privilege-escalation risk.
  nsExec::Exec '"$INSTDIR\randommac.exe" --cleanup-legacy'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Updates reinstall over the existing copy; only a real uninstall removes the autostart task.
  ${If} $UpdateMode <> 1
    nsExec::Exec '"$INSTDIR\randommac.exe" --cleanup-uninstall'
  ${EndIf}
!macroend
