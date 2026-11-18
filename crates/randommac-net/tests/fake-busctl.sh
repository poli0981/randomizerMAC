#!/bin/bash
# Stand-in for busctl(1) next to fake-nmcli.sh: answers NetworkManager's DnsManager.Mode (from
# /run/fake-nm/dns-mode, printed the way `busctl get-property` prints a string) and hands every
# other call to the real busctl, moved to /usr/bin/busctl.real. CI installs it as /usr/bin/busctl.
set -euo pipefail

mode_query="--system --no-pager get-property org.freedesktop.NetworkManager /org/freedesktop/NetworkManager/DnsManager org.freedesktop.NetworkManager.DnsManager Mode"
if [[ "$*" == "$mode_query" ]]; then
  if [[ ! -e /run/fake-nm/dns-mode ]]; then
    echo "Failed to get property Mode on interface org.freedesktop.NetworkManager.DnsManager: Unknown object '/org/freedesktop/NetworkManager/DnsManager'." >&2
    exit 1
  fi
  printf 's "%s"\n' "$(cat /run/fake-nm/dns-mode)"
  exit 0
fi
exec /usr/bin/busctl.real "$@"
