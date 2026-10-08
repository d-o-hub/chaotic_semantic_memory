#!/usr/bin/env bash
# Setup script shim: delegates to scripts/install-hooks.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${SCRIPT_DIR}/install-hooks.sh" "$@"
