#!/bin/bash
# Stand-in for NetworkManager's `nmcli`, used by CI to run the NetworkManager method of the Linux
# backend against a dummy interface (see the "Linux MAC change" job in .github/workflows/ci.yml).
# It answers exactly the calls RandomMac makes (crates/randommac-net/src/linux.rs and
# linux_dns.rs) in the same output format (`-g` values with `:` and `\` escaped), applies
# `cloned-mac-address` the way NetworkManager does when a connection is activated, and computes the
# DNS servers a device uses when a profile is applied (its own servers, then DHCP's unless
# `ignore-auto-dns`). Install it as /usr/bin/nmcli, then run `nmcli fake-setup <ifname>` as root.
#
# State in /run/fake-nm (fixed: RandomMac starts nmcli with an empty environment):
#   device       interface the fake manages
#   active       UUID of the active connection
#   permanent    address used for cloned-mac-address=permanent
#   ignore       if present, activation leaves the address alone (a driver that ignores it)
#   dhcp4/dhcp6  the DNS server "DHCP" hands out
#   effective4/6 the DNS servers the device uses (`IP4.DNS` / `IP6.DNS`)
#   dns-mode     DnsManager.Mode reported through busctl (fake-busctl.sh)
#   dns-reject   if present, `connection modify` accepts DNS servers but keeps the old ones
#   reapply-fail if present, `device reapply` fails (RandomMac then reconnects)
#   connections/<uuid>/{id,cloned,dns4,ignore4,method4,dns6,ignore6,method6,dot}
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

# The DNS servers a device uses once profile $1 is applied; nmcli -g joins them with " | ".
apply_dns() {
  local dir family item joined
  local -a items servers
  dir=$(connection "$1")
  for family in 4 6; do
    servers=()
    IFS=',' read -ra items <<<"$(cat "$dir/dns$family")"
    for item in "${items[@]}"; do
      if [[ -n "$item" ]]; then
        servers+=("${item%%#*}")
      fi
    done
    if [[ "$(cat "$dir/ignore$family")" != yes ]]; then
      servers+=("$(cat "$state/dhcp$family")")
    fi
    joined=""
    for item in "${servers[@]}"; do
      joined+="${joined:+ | }$item"
    done
    printf '%s' "$joined" >"$state/effective$family"
  done
}

# NetworkManager refuses DNS servers on a family configured with one of these methods.
refuses_dns() {
  local method=$1 m
  shift
  for m in "$@"; do
    if [[ "$method" == "$m" ]]; then
      return 0
    fi
  done
  return 1
}

dns_modify() {
  local dir=$1
  shift
  (($# % 2 == 0)) || fail "missing value for the last property."
  local -a args=("$@")
  local i key value method
  # NetworkManager validates the whole change before it writes any of it.
  for ((i = 0; i < ${#args[@]}; i += 2)); do
    key=${args[i]}
    value=${args[i + 1]}
    case "$key" in
      ipv4.dns)
        method=$(cat "$dir/method4")
        if [[ -n "$value" ]] && refuses_dns "$method" disabled link-local; then
          fail "Failed to modify connection: ipv4.dns: this property is not allowed for 'method=$method'"
        fi
        ;;
      ipv6.dns)
        method=$(cat "$dir/method6")
        if [[ -n "$value" ]] && refuses_dns "$method" disabled ignore link-local; then
          fail "Failed to modify connection: ipv6.dns: this property is not allowed for 'method=$method'"
        fi
        ;;
      ipv4.ignore-auto-dns | ipv6.ignore-auto-dns)
        [[ "$value" == yes || "$value" == no ]] || fail "invalid value '$value' for '$key'."
        ;;
      connection.dns-over-tls)
        case "$value" in
          default | no | opportunistic | yes) ;;
          *) fail "invalid value '$value' for '$key'." ;;
        esac
        ;;
      *) fail "invalid property '$key'." ;;
    esac
  done
  for ((i = 0; i < ${#args[@]}; i += 2)); do
    key=${args[i]}
    value=${args[i + 1]}
    case "$key" in
      ipv4.dns | ipv6.dns)
        if [[ ! -e "$state/dns-reject" ]]; then
          printf '%s' "$value" >"$dir/dns${key:3:1}"
        fi
        ;;
      ipv4.ignore-auto-dns | ipv6.ignore-auto-dns) printf '%s' "$value" >"$dir/ignore${key:3:1}" ;;
      connection.dns-over-tls) printf '%s' "$value" >"$dir/dot" ;;
    esac
  done
}

if [[ "${1-}" == fake-setup ]]; then
  device=$2
  mkdir -p "$state/connections/rm-test-a" "$state/connections/rm-test-b"
  echo "$device" >"$state/device"
  echo rm-test-a >"$state/active"
  cat "/sys/class/net/$device/address" >"$state/permanent"
  printf '%s' "Wired connection 1" >"$state/connections/rm-test-a/id"
  printf '%s' "Wired: backup" >"$state/connections/rm-test-b/id"
  for dir in "$state/connections/rm-test-a" "$state/connections/rm-test-b"; do
    : >"$dir/cloned"
    : >"$dir/dns4"
    : >"$dir/dns6"
    printf no >"$dir/ignore4"
    printf no >"$dir/ignore6"
    printf auto >"$dir/method4"
    printf auto >"$dir/method6"
    printf default >"$dir/dot"
  done
  printf 192.0.2.1 >"$state/dhcp4"
  printf 2001:db8::1 >"$state/dhcp6"
  printf systemd-resolved >"$state/dns-mode"
  rm -f "$state/ignore" "$state/dns-reject" "$state/reapply-fail"
  apply_dns rm-test-a
  exit 0
fi

device=$(cat "$state/device")
property=802-3-ethernet.cloned-mac-address
dns_fields=ipv4.dns,ipv4.ignore-auto-dns,ipv4.method,ipv6.dns,ipv6.ignore-auto-dns,ipv6.method,connection.dns-over-tls

case "$*" in
  "-t -f RUNNING general")
    echo running
    ;;
  "-g GENERAL.STATE,GENERAL.CON-UUID device show "*)
    [[ "$5" == "$device" ]] || fail "Device '$5' not found."
    echo "100 (connected)"
    cat "$state/active"
    ;;
  "-g IP4.DNS,IP6.DNS device show "*)
    [[ "$5" == "$device" ]] || fail "Device '$5' not found."
    escape "$(cat "$state/effective4")"
    escape "$(cat "$state/effective6")"
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
  "-g $dns_fields connection show uuid "*)
    dir=$(connection "$6")
    for field in dns4 ignore4 method4 dns6 ignore6 method6; do
      escape "$(cat "$dir/$field")"
    done
    # Printed as a number, written as a name.
    case "$(cat "$dir/dot")" in
      no) echo 0 ;;
      opportunistic) echo 1 ;;
      yes) echo 2 ;;
      *) echo -1 ;;
    esac
    ;;
  "connection modify uuid "*" $property "*)
    dir=$(connection "$4")
    printf '%s' "${6-}" >"$dir/cloned"
    ;;
  "connection modify uuid "*)
    dir=$(connection "$4")
    shift 4
    dns_modify "$dir" "$@"
    ;;
  "device reapply "*)
    [[ "$3" == "$device" ]] || fail "Device '$3' not found."
    if [[ -e "$state/reapply-fail" ]]; then
      fail "Reapplying connection to device '$device' (dummy) failed: Can't reapply changes to 'connection.dns-over-tls' setting"
    fi
    apply_dns "$(cat "$state/active")"
    echo "Connection successfully reapplied to device '$device'."
    ;;
  "--wait 60 connection up uuid "*)
    dir=$(connection "$6")
    echo "$6" >"$state/active"
    if [[ ! -e "$state/ignore" ]]; then
      ip link set dev "$device" address "$(address_for "$device" "$(cat "$dir/cloned")")"
    fi
    apply_dns "$6"
    echo "Connection successfully activated"
    ;;
  *)
    echo "fake nmcli: unsupported arguments: $*" >&2
    exit 2
    ;;
esac
