#!/usr/bin/env bash
# Wraps command execution with CSM_LD_LIBRARY_PATH exported as LD_LIBRARY_PATH.
# This prevents dynamic linking errors on Nix-built binaries (like libstdc++.so.6)
# while avoiding pollution of the devShell's global environment.

set -euo pipefail

if [[ -n "${CSM_LD_LIBRARY_PATH:-}" ]]; then
  export LD_LIBRARY_PATH="${CSM_LD_LIBRARY_PATH}"
fi

exec "$@"
