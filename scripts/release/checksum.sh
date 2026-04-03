#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <output-file> <artifact> [artifact ...]" >&2
  exit 1
fi

output_file="$1"
shift

mkdir -p "$(dirname "$output_file")"
: > "$output_file"

for artifact in "$@"; do
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$artifact" >> "$output_file"
  else
    shasum -a 256 "$artifact" >> "$output_file"
  fi
done
