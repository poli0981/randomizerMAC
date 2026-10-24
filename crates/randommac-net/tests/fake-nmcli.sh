#!/bin/bash
# Stand-in for NetworkManager's `nmcli`, used by CI to run the NetworkManager method of the Linux
# backend against a dummy interface (see the "Linux MAC change" job in .github/workflows/ci.yml).
# It answers exactly the calls RandomMac makes (crates/randommac-net/src/linux.rs) in the same
# output format (`-g` values with `:` and `\` escaped) and applies `cloned-mac-address` the way
# NetworkManager does when a connection is activated. Install it as /usr/bin/nmcli, then run
# `nmcli fake-setup <ifname>` as root.
#
# State in /run/fake-nm (fixed: RandomMac starts nmcli with an empty environment):
#   device       interface the fake manages
#   active       UUID of the active connection
#   permanent    address used for cloned-mac-address=permanent
#   ignore       if present, activation leaves the address alone (a driver that ignores it)
#   connections/<uuid>/{id,cloned}
set -euo pipefail

state=/run/fake-nm

fail() {
  echo "Error: $*" >&2
  exit 10
}

escape() {
  sed 's/[\\:]/\\&/g' <<<"$1"
}

connection() {
  [[ -d "$state/connections/$1" ]] || fail "unknown connection '$1'."
  echo "$state/connections/$1"
}

# Address NetworkManager sets when it activates a profile with this cloned-mac-address.
address_for() {
  local device=$1 value=$2
  case "$value" in
    "" | preserve) cat "/sys/class/net/$device/address" ;;
    permanent) cat "$state/permanent" ;;
    stable) echo "02:5a:5a:5a:5a:5a" ;;
    random) printf '02:%02x:%02x:%02x:%02x:%02x\n' $((RANDOM % 256)) $((RANDOM % 256)) \
      $((RANDOM % 256)) $((RANDOM % 256)) $((RANDOM % 256)) ;;
    *) echo "$value" ;;
  esac
}

if [[ "${1-}" == fake-setup ]]; then
  device=$2
  mkdir -p "$state/connections/rm-test-a" "$state/connections/rm-test-b"
  echo "$device" >"$state/device"
  echo rm-test-a >"$state/active"
  cat "/sys/class/net/$device/address" >"$state/permanent"
  printf '%s' "Wired connection 1" >"$state/connections/rm-test-a/id"
  printf '%s' "Wired: backup" >"$state/connections/rm-test-b/id"
  : >"$state/connections/rm-test-a/cloned"
  : >"$state/connections/rm-test-b/cloned"
  rm -f "$state/ignore"
  exit 0
fi

device=$(cat "$state/device")
property=802-3-ethernet.cloned-mac-address

case "$*" in
  "-t -f RUNNING general")
    echo running
    ;;
  "-g GENERAL.STATE,GENERAL.CON-UUID device show "*)
    [[ "$5" == "$device" ]] || fail "Device '$5' not found."
    echo "100 (connected)"
    cat "$state/active"
    ;;
  "-g connection.id,connection.type connection show uuid "*)
    dir=$(connection "$6")
    escape "$(cat "$dir/id")"
    echo 802-3-ethernet
    ;;
  "-g $property connection show uuid "*)
    dir=$(connection "$6")
    escape "$(cat "$dir/cloned")"
    ;;
  "connection modify uuid "*" $property "*)
    dir=$(connection "$4")
    printf '%s' "${6-}" >"$dir/cloned"
    ;;
  "--wait 60 connection up uuid "*)
    dir=$(connection "$6")
    echo "$6" >"$state/active"
    if [[ ! -e "$state/ignore" ]]; then
      ip link set dev "$device" address "$(address_for "$device" "$(cat "$dir/cloned")")"
    fi
    echo "Connection successfully activated"
    ;;
  *)
    echo "fake nmcli: unsupported arguments: $*" >&2
    exit 2
    ;;
esac
