#!/usr/bin/env bash
# Create LANtern post-MVP GitHub issues. Requires `gh` auth with issues:write.
set -euo pipefail

create_issue() {
  local title="$1"
  local body="$2"
  echo "Creating: $title"
  gh issue create --title "$title" --label "enhancement" --body "$body"
}

create_issue "Ping tool for individual devices" "$(cat <<'BODY'
## Summary
Add an ICMP (and TCP fallback) ping tool reachable from a device detail view.

## Acceptance criteria
- [ ] Ping a selected device from the UI
- [ ] Show latency, packet loss, and a short history for the session
- [ ] Work without requiring the app to always run as admin when possible; document elevation cases
- [ ] Expose a Rust command such as `ping_device(ip)` for the frontend

## Context
Part of the post-MVP LANtern toolkit (classic Fing-style device tools).
BODY
)"

create_issue "TCP port scanner for discovered devices" "$(cat <<'BODY'
## Summary
Add a TCP port scan against a selected device, with sensible defaults and custom port ranges.

## Acceptance criteria
- [ ] Scan common ports by default (e.g. top services)
- [ ] Allow a custom port list/range
- [ ] Show open/closed/filtered results in the device detail UI
- [ ] Bound concurrency and timeouts so scans stay responsive
- [ ] Expose a Rust command such as `scan_ports(ip, ports)`

## Context
Planned follow-up to core LAN discovery in LANtern.
BODY
)"

create_issue "Wake-on-LAN support" "$(cat <<'BODY'
## Summary
Send Wake-on-LAN magic packets to devices with a known MAC address.

## Acceptance criteria
- [ ] WoL action on device detail when a MAC is available
- [ ] Broadcast magic packet on the local network
- [ ] Clear UI feedback for sent / failed
- [ ] Expose a Rust command such as `wake_on_lan(mac)`
- [ ] Document NIC/BIOS requirements for WoL to succeed

## Context
Planned LANtern toolkit feature for powering on sleeping hosts.
BODY
)"

create_issue "DHCP test tool" "$(cat <<'BODY'
## Summary
Add a DHCP test utility to validate DHCP server behavior on the local network (discover/offer/request/ack style checks).

## Acceptance criteria
- [ ] Initiate a DHCP discovery test from the app
- [ ] Report whether a server responds, and surface offer details (server IP, offered address, lease time, gateway/DNS when present)
- [ ] Make privilege/raw-socket requirements explicit in the UI and docs
- [ ] Avoid disrupting the machine’s active lease by default (test mode / careful packet handling)
- [ ] Surface failures clearly (no response, NAK, timeout)

## Notes
This is a diagnostics tool, not a DHCP server. Implementation will likely need platform-specific raw socket or BPF access.

## Context
Requested as a later LANtern networking utility beyond core device discovery.
BODY
)"

create_issue "Mobile support via Tauri" "$(cat <<'BODY'
## Summary
Bring LANtern to iOS/Android using Tauri mobile once the desktop toolkit is solid.

## Acceptance criteria
- [ ] Shared Rust scanning core where platform APIs allow
- [ ] Mobile-friendly UI for scan + device list
- [ ] Document platform permission requirements (local network, etc.)
- [ ] Feature parity expectations clearly scoped (some desktop tools may remain desktop-only)

## Context
Explicitly deferred from the desktop MVP; desktop remains the priority.
BODY
)"

echo "Done."
