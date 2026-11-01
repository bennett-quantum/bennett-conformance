#!/usr/bin/env bash
set -euo pipefail
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS BROWSER PYTHONOPTIMIZE
root="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
export TMPDIR="$(realpath "$root/../..")/.tmp/rung-4/tmp"
mkdir -p "$TMPDIR"
exec python3 "$root/gate.py" "$@"
