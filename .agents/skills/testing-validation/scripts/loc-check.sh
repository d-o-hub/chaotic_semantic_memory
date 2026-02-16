#!/usr/bin/env bash
set -euo pipefail

fail=0
for file in src/*.rs; do
  loc=$(wc -l < "$file")
  if [ "$loc" -gt 500 ]; then
    echo "FAIL $file: $loc LOC (max 500)"
    fail=1
  else
    echo "OK   $file: $loc LOC"
  fi
done

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "All files within LOC limits."
