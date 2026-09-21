#!/usr/bin/env bash
# scripts/wasm-evidence.sh
#
# Tier-3 evidence for the npm artifact (ADR-0095): "package-size and JS runtime
# smoke evidence for the exact npm artifact". Produces a manifest plus the raw
# smoke-test transcript for the package `release.yml` publishes, so a reader can
# see the bytes, the hashes and the runtime result in one place.
#
# Usage:
#   scripts/wasm-evidence.sh [OUT_DIR] [--package-dir DIR]
#
# Default OUT_DIR: plans/evidence/wasm_<UTC date>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

OUT_DIR=""
PACKAGE_DIR=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --package-dir) PACKAGE_DIR="$2"; shift 2 ;;
    --package-dir=*) PACKAGE_DIR="${1#*=}"; shift ;;
    *) OUT_DIR="$1"; shift ;;
  esac
done

if [[ -z "${OUT_DIR}" ]]; then
  OUT_DIR="plans/evidence/wasm_$(date -u +%Y%m%d)"
fi
mkdir -p "${OUT_DIR}"

BUILD_MODE="release-web (scripts/build-wasm.sh)"
if [[ -z "${PACKAGE_DIR}" ]]; then
  PACKAGE_DIR="${OUT_DIR}/pkg"
  echo "==> building the release artifact (scripts/build-wasm.sh release-web)"
  bash scripts/build-wasm.sh release-web "${PACKAGE_DIR}" >/dev/null
else
  echo "==> measuring existing package: ${PACKAGE_DIR}"
  BUILD_MODE="existing package (no build)"
fi

echo "==> JS runtime smoke test"
SMOKE_LOG="${OUT_DIR}/smoke.log"
set +e
WASM_PACKAGE_DIR="${PACKAGE_DIR}" node wasm/test.js > "${SMOKE_LOG}" 2>&1
SMOKE_STATUS=$?
set -e
tail -3 "${SMOKE_LOG}"

WASM_FILE="${PACKAGE_DIR}/chaotic_semantic_memory_bg.wasm"
GLUE_FILE="${PACKAGE_DIR}/chaotic_semantic_memory.js"
DTS_FILE="${PACKAGE_DIR}/chaotic_semantic_memory.d.ts"
for f in "${WASM_FILE}" "${GLUE_FILE}" "${DTS_FILE}"; do
  [[ -f "$f" ]] || { echo "missing artifact: $f" >&2; exit 1; }
done

WASM_BYTES="$(wc -c < "${WASM_FILE}")"
WASM_SHA="$(sha256sum "${WASM_FILE}" | cut -d' ' -f1)"
GLUE_BYTES="$(wc -c < "${GLUE_FILE}")"
GLUE_SHA="$(sha256sum "${GLUE_FILE}" | cut -d' ' -f1)"
DTS_BYTES="$(wc -c < "${DTS_FILE}")"
DTS_SHA="$(sha256sum "${DTS_FILE}" | cut -d' ' -f1)"
VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' wasm/package.json)"

export EVIDENCE_COMMIT EVIDENCE_DIRTY EVIDENCE_OUT_DIR EVIDENCE_PACKAGE_DIR
EVIDENCE_COMMIT="$(git rev-parse HEAD)"
EVIDENCE_DIRTY=false
if [[ -n "$(git status --porcelain -- . ':!plans/evidence')" ]]; then
  EVIDENCE_DIRTY=true
fi
EVIDENCE_OUT_DIR="${OUT_DIR}"
EVIDENCE_PACKAGE_DIR="${PACKAGE_DIR}"
export EVIDENCE_WASM_BYTES="${WASM_BYTES}" EVIDENCE_WASM_SHA="${WASM_SHA}"
export EVIDENCE_GLUE_BYTES="${GLUE_BYTES}" EVIDENCE_GLUE_SHA="${GLUE_SHA}"
export EVIDENCE_DTS_BYTES="${DTS_BYTES}" EVIDENCE_DTS_SHA="${DTS_SHA}"
export EVIDENCE_VERSION="${VERSION}" EVIDENCE_SMOKE_STATUS="${SMOKE_STATUS}"
export EVIDENCE_BUILD_MODE="${BUILD_MODE}"

python3 - <<'PY'
import json
import os
import pathlib
import subprocess

out = pathlib.Path(os.environ["EVIDENCE_OUT_DIR"])


def run(*args: str) -> str:
    return subprocess.run(list(args), capture_output=True, text=True, check=False).stdout.strip()


cpu = next(
    (line.split(":", 1)[1].strip() for line in pathlib.Path("/proc/cpuinfo").read_text().splitlines()
     if line.startswith("model name")),
    "unknown",
)
manifest = {
    "schema_version": 1,
    "artifact": "wasm_npm_package",
    "commit": os.environ["EVIDENCE_COMMIT"],
    "dirty": os.environ["EVIDENCE_DIRTY"] == "true",
    "command": f"scripts/wasm-evidence.sh {os.environ['EVIDENCE_OUT_DIR']}",
    "build": {
        "mode": os.environ["EVIDENCE_BUILD_MODE"],
        "target": "web",
        "wasm_opt": "from [package.metadata.wasm-pack.profile.release]",
    },
    "package": {
        "name": "@d-o-hub/chaotic_semantic_memory",
        "version": os.environ["EVIDENCE_VERSION"],
        "wasm_bytes": int(os.environ["EVIDENCE_WASM_BYTES"]),
        "wasm_sha256": os.environ["EVIDENCE_WASM_SHA"],
        "js_bytes": int(os.environ["EVIDENCE_GLUE_BYTES"]),
        "js_sha256": os.environ["EVIDENCE_GLUE_SHA"],
        "dts_bytes": int(os.environ["EVIDENCE_DTS_BYTES"]),
        "dts_sha256": os.environ["EVIDENCE_DTS_SHA"],
    },
    "smoke": {
        "command": "node wasm/test.js",
        "exit_code": int(os.environ["EVIDENCE_SMOKE_STATUS"]),
        "transcript": "smoke.log",
    },
    "toolchain": run("rustc", "--version"),
    "hardware": {
        "cpu": cpu,
        "os": f"{run('uname', '-s')} {run('uname', '-r')}",
        "arch": run("uname", "-m"),
    },
}
(out / "evidence.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"wrote {out / 'evidence.json'}")
PY

if [[ "${SMOKE_STATUS}" -ne 0 ]]; then
  echo "smoke test failed (exit ${SMOKE_STATUS}); see ${SMOKE_LOG}" >&2
  exit 1
fi
echo "wasm evidence complete: ${OUT_DIR}"
