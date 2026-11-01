#!/usr/bin/env bash
# Fetch the pinned SQLite amalgamation into the gate's cache. Data only: gate.py verifies the
# published SHA3-256, size and SHA-256 and compiles it; nothing in the archive ever runs.
set -euo pipefail
root="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.."
read -r url zip < <(python3 -I -c 'import json,sys;p=json.load(open(sys.argv[1]))["sqlite"];print(p["url"],p["url"].rsplit("/",1)[1])' "$root/pins.json")
dest="$(realpath "$root/../..")/.tmp/rung-4/sqlite"
mkdir -p "$dest"
[ -f "$dest/$zip" ] || curl -fsS --retry 5 --max-time 180 -o "$dest/$zip" "$url"
ls -l "$dest/$zip"
