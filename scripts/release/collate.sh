#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

find "$dist_dir" -name 'checksums-*.txt' -type f -exec cat {} + > "$tmp_file"
sort -k2 "$tmp_file" | uniq > "$dist_dir/checksums.txt"
