#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version-tag>" >&2
  exit 1
fi

version="$1"
if [[ ! "$version" =~ ^v0\.1\.0-beta\.[0-9]+$ ]]; then
  echo "only long-lived beta tags v0.1.0-beta.N are allowed" >&2
  exit 1
fi

crate_version="$(awk '
  BEGIN { in_package=0 }
  /^\[package\]/ { in_package=1; next }
  /^\[/ && in_package { exit }
  in_package && $0 ~ /^version[[:space:]]*=/ {
    gsub(/"/, "", $0)
    sub(/^version[[:space:]]*=[[:space:]]*/, "", $0)
    print $0
    exit
  }
' app/Cargo.toml)"

if [[ "v${crate_version}" != "$version" ]]; then
  echo "tag/version mismatch: got ${version}, expected v${crate_version}" >&2
  exit 1
fi

printf '%s\n' "$version"
