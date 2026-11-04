#!/usr/bin/env bash
# Mutation testing wrapper (cargo-mutants).
#
# Profiles:
#   fast (default, CI): --in-diff vs base, unit tests only (--lib), tight timeouts
#   full: entire tree, full cargo test suite (local/nightly use)
#
# Performance notes (2026-07-16, PR #516 analysis of job 87601199764):
#   - Full `cargo test` ≈ 12s unit + ~97s integration per mutant.
#   - 120 in-diff mutants × ~2 min / 4 jobs ≈ 60+ min wall time.
#   - Fast CI uses `-- --lib` so each mutant is ~10–15s of tests after build.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CI_MODE=false
THRESHOLD="${MUTATION_THRESHOLD:-85}"
POSITIONAL=()
NO_DEFAULT_FEATURES=false
# Set true only when the bounded diff selects at least one production Rust file.
# It is what separates "this PR has nothing to mutate" from "the gate broke".
DIFF_RUST_SELECTED=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ci)
      CI_MODE=true
      shift
      ;;
    --threshold=*)
      THRESHOLD="${1#*=}"
      shift
      ;;
    --threshold)
      THRESHOLD="$2"
      shift 2
      ;;
    --no-default-features)
      NO_DEFAULT_FEATURES=true
      shift
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

PROFILE="${1:-fast}"
shift || true

# Issue #951: each CI shard runs one disjoint slice and uploads its outcomes; the
# `mutation-test` aggregator owns the threshold over the union. A shard must never
# enforce the threshold on its own subset (an all-missed shard would otherwise be
# rescued by a sibling, or vice versa).
#
# MUTANTS_SHARD is 1-based because the completeness markers and the aggregator's
# `1..N` completeness check are. cargo-mutants is not: it numbers shards `0..N-1`
# and rejects `k == N`, so a pass-through `8/8` dies in its argument parser while
# `1/8` selects its *second* slice — shard 0's mutants then belong to no job that
# runs, and the union scores a denominator that never contained them. Translate
# only at the call site below; the marker keeps the caller's id.
SHARD="${MUTANTS_SHARD:-}"
SHARD_INDEX=0
SHARD_TOTAL=0
if [[ -n "${SHARD}" ]]; then
  if [[ ! "${SHARD}" =~ ^([0-9]+)/([0-9]+)$ ]]; then
    echo "error: MUTANTS_SHARD must look like i/N, got '${SHARD}'" >&2
    exit 2
  fi
  SHARD_INDEX=$((10#"${BASH_REMATCH[1]}"))
  SHARD_TOTAL=$((10#"${BASH_REMATCH[2]}"))
  if (( SHARD_TOTAL < 1 || SHARD_INDEX < 1 || SHARD_INDEX > SHARD_TOTAL )); then
    echo "error: MUTANTS_SHARD '${SHARD}' is outside 1..${SHARD_TOTAL} of ${SHARD_TOTAL}" >&2
    exit 2
  fi
fi

# Parallelism:
# - fast + in-diff (normal case): JOBS=1 to reuse sequential build artifacts across mutants.
# - fast + full-tree fallback (no diff detected): escalate to MUTANTS_JOBS (default 4) so
#   1 300+ mutants do not burn the full CI budget at 1 job.
# - full profile: use MUTANTS_JOBS (default 1 locally).
FAST_FALLBACK_JOBS="${MUTANTS_JOBS:-4}"
if [[ "${PROFILE}" == "fast" ]]; then
  JOBS=1          # overridden below if diff is empty
else
  JOBS="${MUTANTS_JOBS:-1}"
fi

if ! command -v cargo-mutants &>/dev/null && ! cargo mutants --version &>/dev/null; then
  cat <<'MSG' >&2
cargo-mutants is not installed.
Install it with:
  cargo install cargo-mutants
Or in CI with:
  - uses: taiki-e/install-action@v2
    with:
      tool: cargo-mutants
MSG
  exit 127
fi

OUT_DIR="${MUTATION_OUTPUT_DIR:-progress/mutation}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${OUT_DIR}/${PROFILE}-${TIMESTAMP}.log"
REPORT_FILE="${OUT_DIR}/${PROFILE}-latest.md"
PREFLIGHT_LOG="${OUT_DIR}/${PROFILE}-preflight-latest.log"
mkdir -p "${OUT_DIR}"
TMP_PKGS="$(mktemp)"
trap 'rm -f "${TMP_PKGS}"' EXIT

HELP_TEXT="$(cargo mutants --help 2>/dev/null || true)"
FAST_ARGS=()
TEST_ARGS=()
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Packages the fast profile always inspects, whether or not the diff touches them:
# `--features chaotic_semantic_memory/mcp` (below) is only valid when the root
# package is selected, so the selection can never be empty.
ROOT_PACKAGE="chaotic_semantic_memory"
MUTATION_PACKAGES=(csm-core-lib csm-retrieval "${ROOT_PACKAGE}")

# Map the diff's production Rust files to the packages that own them.
#
# WHY THIS EXISTS (2026-11-01, PR #977): `cargo mutants` with no `-p` reads the
# *current package*, and the root manifest is both a package and a workspace, so
# discovery walked `src/**` only — 1259 mutants, zero of them under `crates/`,
# measured on the #977 head. `detect-changes` (ci.yml:127) nevertheless sets
# `mutation=true` for `crates/*/src/*.rs`, and TEST_ARGS already tested
# csm-core-lib and csm-retrieval, so the gate claimed the diff was
# mutation-visible and then proved it with a scope that could not see it. This is
# the same fault ci-workspace-crate-matrix.sh:14-17 documents for the
# lint/test jobs as issue #905 — third occurrence, and the mutation job was the
# one still unfixed.
#
# Returns 2 when the workspace cannot be described or a selected file belongs to
# no package: an unresolved scope is a broken gate, never a passing one.
diff_package_scope() {
  local diff_file="$1"
  local metadata abs best best_pkg file pkg root
  if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq is not installed; the mutation gate cannot map the diff to its packages" >&2
    return 2
  fi
  if ! metadata="$(cargo metadata --no-deps --format-version 1 \
      --manifest-path "${REPO_ROOT}/Cargo.toml" 2>&1)"; then
    echo "error: cargo metadata failed; the mutation gate cannot resolve which packages the diff touches" >&2
    printf '%s\n' "${metadata}" | sed 's/^/  /' >&2
    return 2
  fi
  # "<package root dir>\t<package name>" per line. An empty table must not read as
  # "nothing in scope", so the parse and the result are both checked.
  if ! printf '%s' "${metadata}" \
      | jq -r '.packages[] | [.manifest_path, .name] | @tsv' \
      | sed 's|/Cargo\.toml\t|\t|' >"${TMP_PKGS}"; then
    echo "error: could not parse cargo metadata into a package table" >&2
    return 2
  fi
  if [[ ! -s "${TMP_PKGS}" ]]; then
    echo "error: cargo metadata reported no workspace packages" >&2
    return 2
  fi

  while IFS= read -r file; do
    if [[ -z "${file}" ]]; then
      continue
    fi
    abs="${REPO_ROOT}/${file}"
    best=""
    best_pkg=""
    while IFS=$'\t' read -r root pkg; do
      case "${abs}" in
        "${root}/"*)
          if [[ ${#root} -gt ${#best} ]]; then
            best="${root}"
            best_pkg="${pkg}"
          fi
          ;;
      esac
    done <"${TMP_PKGS}"
    if [[ -z "${best_pkg}" ]]; then
      echo "error: ${file} is a selected Rust source but no workspace package owns it" >&2
      return 2
    fi
    printf '%s\n' "${best_pkg}"
  done < <(
    # The same path rule ci.yml:127 uses to decide this job is required; a diff
    # that edits no production Rust has no mutants by construction.
    grep -E '^\+\+\+ b/' "${diff_file}" | sed 's|^+++ b/||' \
      | grep -E '^(src/.*\.rs|crates/[^/]+/src/.*\.rs)$' \
      | grep -Ev '(^|/)(tests/|tests\.rs$|[^/]*_tests\.rs$)' | sort -u
  )
  return 0
}

if [[ "${PROFILE}" == "fast" ]]; then
  if grep -q -- '--in-diff' <<<"${HELP_TEXT}"; then
    DIFF_TARGET="${DIFF_TARGET:-origin/main}"
    DIFF_FILE="$(mktemp)"
    trap 'rm -f "${TMP_PKGS}" "${DIFF_FILE}"' EXIT
    # Three-dot: only commits on this branch since merge-base (true PR delta).
    # -C REPO_ROOT so the paths in the diff are workspace-root-relative, which is
    # what both cargo-mutants' --in-diff filter and the package scope below assume.
    if git -C "${REPO_ROOT}" rev-parse --verify "${DIFF_TARGET}" >/dev/null 2>&1; then
      git -C "${REPO_ROOT}" diff "${DIFF_TARGET}...HEAD" >"${DIFF_FILE}" 2>/dev/null \
        || git -C "${REPO_ROOT}" diff "${DIFF_TARGET}" >"${DIFF_FILE}" 2>/dev/null \
        || true
    else
      git -C "${REPO_ROOT}" diff "${DIFF_TARGET}" >"${DIFF_FILE}" 2>/dev/null || true
    fi
    if [[ -s "${DIFF_FILE}" ]]; then
      # Issue #952: a same-file native/wasm cfg pair in the changed lines puts an
      # unkillable mutant in this profile's denominator. Fail before the sweep
      # spends an hour proving it; the fixture proves both directions.
      if ! python3 "${SCRIPT_DIR}/check_cfg_mutants.py" "${REPO_ROOT}" --diff - < "${DIFF_FILE}"; then
        echo "error: the diff adds a cfg-split declaration the native --lib profile cannot kill" >&2
        exit 1
      fi
      FAST_ARGS+=(--in-diff "${DIFF_FILE}")
      echo "mutation fast: in-diff against ${DIFF_TARGET} ($(wc -l <"${DIFF_FILE}") diff lines)" >&2
      # Scope mutant *generation* to the packages the diff touches. Without this
      # the job tests the root package and calls that coverage of a crates/** diff.
      # Captured, not piped: a process substitution would drop the function's exit
      # status and turn an unresolved scope into a silent empty list.
      if ! DIFF_SCOPE_OUT="$(diff_package_scope "${DIFF_FILE}")"; then
        echo "error: mutation scope for the selected diff could not be resolved" >&2
        exit 1
      fi
      if [[ -n "${DIFF_SCOPE_OUT}" ]]; then
        DIFF_RUST_SELECTED=true
      fi
      read -r -a DIFF_PACKAGES <<< "${DIFF_SCOPE_OUT}"
      for diff_pkg in "${DIFF_PACKAGES[@]}"; do
        if ! printf '%s\n' "${MUTATION_PACKAGES[@]}" | grep -qxF "${diff_pkg}"; then
          MUTATION_PACKAGES+=("${diff_pkg}")
          echo "mutation fast: diff selects package ${diff_pkg} — added to the mutation scope" >&2
        fi
      done
      # Feature-aware mutation: cfg(not(feature = "persistence")) mutants are
      # unreachable under default features — both generation and tests must run
      # with the feature off (ADR-0094, PR #621).
      if grep -Eq '^\+.*cfg\(not\(feature = "persistence"\)\)' "${DIFF_FILE}"; then
        NO_DEFAULT_FEATURES=true
        echo 'mutation fast: diff contains cfg(not(feature = "persistence")) — running with --no-default-features' >&2
      fi
    elif [[ "${CI_MODE}" == "true" ]]; then
      # An unbounded main-branch run is intentionally disabled. This invocation
      # produced no mutation evidence, so it must not succeed as a test result.
      echo "error: no bounded diff against ${DIFF_TARGET}; mutation tests did not run" >&2
      {
        echo "# Unit-Target Mutation Test Report"
        echo
        echo "- Timestamp (UTC): ${TIMESTAMP}"
        echo "- Profile: ${PROFILE}"
        echo "- Mutants: 0"
        echo "- Result: not run (no bounded diff; not a passing mutation result)"
      } >"${REPORT_FILE}"
      exit 2
    else
      echo "warning: no diff against ${DIFF_TARGET}; running full target set" >&2
      # Full-tree fallback (local/nightly only): escalate parallelism so the run finishes.
      # JOBS=1 is only safe for the in-diff case (sequential artifact reuse).
      JOBS="${FAST_FALLBACK_JOBS}"
      echo "mutation fast: full-tree fallback — escalating to JOBS=${JOBS}" >&2
    fi
  else
    echo "warning: --in-diff is unsupported by installed cargo-mutants; running full target set" >&2
  fi

  # Unit-target tests only (--lib): integration suite dominates wall time (~8× unit suite).
  # Any change to src/** or crates/** must have unit test assertions in the lib target
  # to be visible to this mutation gate (process-level tests in tests/** are ignored).
  # MUTATION_PACKAGES starts at csm-core-lib, csm-retrieval and chaotic_semantic_memory to
  # avoid building irrelevant workspace packages (up to 80% of build time), and grows with
  # whatever the diff touches — so a new member is covered without anyone editing this list.
  # The same list reaches mutant generation, split into arms (see ROOT_PACKAGE below): a
  # scope that only the test side had is how PR #977 went red on a job that could not see
  # its code, and one flat `cargo mutants -p A -p B --features pkg/feat` call is how it
  # then dies, because cargo-mutants builds one `cargo test --package=X` per selected
  # package and a package-qualified feature is invalid in every other package's command
  # (measured on the #977 head: `--package=csm-core-lib@0.3.9
  # --features=chaotic_semantic_memory/mcp` → "does not contain this feature" →
  # "cargo build failed in an unmutated tree, so no mutants were tested").
  # The package scope lives on the *arm*, not here: cargo-mutants already forwards its
  # own `-p` selection into every `cargo test --package=X` it builds, so repeating the
  # union on the test side is redundant — and with feature arms it is actively wrong.
  # Measured on the #998 diff: the `feat-csm-chaos` arm ran
  # `--all-features … -- --lib -p csm-core-lib -p csm-retrieval -p chaotic_semantic_memory
  # -p csm-chaos`, which activates the features of all four packages at once, and its
  # unmutated baseline hit `*** result: Timeout` (exit 4, "cargo test failed in an
  # unmutated tree, so no mutants were tested") in both feature arms. Scoped to the arm
  # the same baseline builds in seconds. This is #1005's fault — the test-side scope is
  # arm-agnostic — seen from the arm side.
  TEST_ARGS+=(--lib)

  # `mcp` is not in the crate's `default` feature set (Cargo.toml:
  # default = ["cli", "persistence", "parallel"]) and both
  # `chaotic_semantic_memory::mcp` (src/lib.rs:57) and `crate::cli::mcp`
  # (src/cli/mod.rs:6) are `#[cfg(feature = "mcp")]`. Under the default-feature
  # profile that code is never compiled, so a mutation in it changed nothing
  # observable and cargo-mutants reported it as MISSED — which reads like "the
  # tests failed to kill it" but means "the job could not see it at all".
  # Measured with the same cargo-mutants 27.1.0 CI uses, on the #817 diff
  # (--in-diff, --lib, jobs=1, no build bound): both `src/cli/mcp.rs:42`
  # parse-helper mutants flipped MISSED -> CAUGHT (killed by the `45` assertion
  # in the same module) and the base build went to 146-183s on a target that had
  # to link the feature graph (23s test). Two costs are therefore paid here:
  # rmcp/axum/tower join every mutation job's base build, and that build needs
  # both the raised `--build-timeout` in the CI block below (at 150s its own
  # mutants came back TIMEOUT and the score printed 0% on a change that is
  # otherwise killable) and the ci.yml pre-warm step that relinks them once
  # before the run — after warming, touching one file and rebuilding both
  # feature-enabled test binaries measured 39s. The *path* set is now as wide as
  # the feature set: `src/mcp/*` left this job's scope on 2026-10-15 (#833), which
  # ran the module's first baseline and added the lib-target tests and the
  # documented excludes below — evidence in
  # `plans/evidence/mutation_mcp_2026_10_15/`.
  #
  # It is applied to the ROOT ARM ONLY. `chaotic_semantic_memory/mcp` gates code that
  # lives in the root package (`src/lib.rs:57`, `src/cli/mcp.rs`), and passing it to an
  # arm that also selects `crates/*` poisons cargo-mutants' per-package build command.
  MCP_FEATURE="${ROOT_PACKAGE}/mcp"

  if [[ "${NO_DEFAULT_FEATURES}" == "true" ]]; then
    # --cargo-arg applies --no-default-features to every cargo invocation
    # (build AND test); adding it again in TEST_ARGS would make cargo reject
    # the duplicated `--no-default-features` flag on the test invocation.
    FAST_ARGS+=(--cargo-arg=--no-default-features)
  fi

  if [[ "${CI_MODE}" == "true" ]]; then
    # --in-place is incompatible with parallel jobs (-j)
    if [[ "${JOBS}" -eq 1 ]]; then
      FAST_ARGS+=(--in-place)
    fi
    FAST_ARGS+=(--no-shuffle)
    # Tight bounds: kill hung mutants; keep build-timeout short so pathological
    # const/eval mutants don't burn the full job budget (was 180s, then 150s).
    # 150s is now measured to be too short for an *incremental* build: with the
    # default features those rebuilds ran ~54s, but adding
    # chaotic_semantic_memory/mcp put the unmutated baseline at 183s on a warm
    # target and individual mutants at 145-208s, so three otherwise killable
    # mutants came back TIMEOUT and the score printed 0% (the same three are
    # CAUGHT with no build bound). 420s keeps the pathological-mutant guard — a
    # single mutant still cannot eat the job's 45 minutes — while leaving the
    # feature-enabled build room. The cold case is handled where it belongs, by
    # pre-warming the feature build in ci.yml, not by a loose per-mutant bound.
    FAST_ARGS+=(--timeout 120)
    FAST_ARGS+=(--minimum-test-timeout 15)
    FAST_ARGS+=(--build-timeout 420)
  else
    FAST_ARGS+=(--timeout 90)
    FAST_ARGS+=(--minimum-test-timeout 15)
    FAST_ARGS+=(--build-timeout 120)
  fi
elif [[ "${PROFILE}" != "full" ]]; then
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  exit 2
else
  # full profile: broader timeouts, full suite (no --lib)
  #
  # KNOWN BLIND SPOT, left open deliberately: no `-p`/`--workspace` reaches
  # FAST_ARGS here, so the nightly sweep mutates the root package only — the same
  # current-package fault that red-gated PR #977. Widening it is not a two-line
  # edit: csm-duckdb links bundled libduckdb and csm-wasm is wasm32-only, so the
  # sweep needs a per-package scope decision and a shard budget, which is issue
  # #951's work (sharding behind the one required `mutation-test` context). Until
  # that lands, "Full Mutation Sweep" in scheduled-evidence.yml means "full tree of
  # the root package", and this line is where a reader can find out.
  FAST_ARGS+=(--timeout 120)
  FAST_ARGS+=(--minimum-test-timeout 30)
  FAST_ARGS+=(--build-timeout 180)
fi

if [[ -n "${SHARD}" ]]; then
  FAST_ARGS+=(--shard "$((SHARD_INDEX - 1))/${SHARD_TOTAL}")
fi

MUTANTS_ARGS=("${FAST_ARGS[@]}")
if [[ "${JOBS}" -gt 1 ]]; then
  MUTANTS_ARGS+=(-j "${JOBS}")
  # --in-place must not be present when -j > 1 (cargo-mutants restriction).
  # We filter --in-place out of MUTANTS_ARGS to avoid conflicts.
  FILTERED_ARGS=()
  for arg in "${MUTANTS_ARGS[@]}"; do
    if [[ "$arg" != "--in-place" ]]; then
      FILTERED_ARGS+=("$arg")
    fi
  done
  MUTANTS_ARGS=("${FILTERED_ARGS[@]}")
fi

# Shared excludes: I/O, WASM stubs, uninteresting Drop/guards.
EXCLUDE_ARGS=(
  --exclude-re "WasmFramework::"
  --exclude-re "persistence::"
  --exclude-re "HnswIndex::serialize"
  --exclude-re "HnswIndex::deserialize"
  --exclude-re "OtlpGuard::"
  --exclude-re "install_grpc_tracer"
  --exclude-re "impl Drop for Guard"
  --exclude-re "impl Drop for OtlpGuard"
  --exclude-re "Result<Option<Guard>>"
  --exclude-re "replace && with"
  --exclude-re "delete . in init"
  --exclude-re "replace > with >= in FrameworkBuilder::with_max_associations_per_concept"
  --exclude-re "replace > with .* in FrameworkBuilder::build"
  --exclude-re "ChaoticSemanticFramework::load "
  --exclude "src/persistence_wasm.rs"
  --exclude-re "replace > with >= in <impl Reranker for MmrReranker>::rerank"
  # Hybrid merge equivalent mutants (2026-09-13, PR #693; proof in
  # plans/.archive/2026-11-03-historical/PR_ROAST_2026_09_12.md): select_nth(top_k - 1) + truncate(top_k)
  # makes `- -> /` (top_k/1 == the pre-2026-09 form) and `> -> >=` (wider
  # partition, same post-truncate top-K set) produce identical output for
  # every input — unkillable by any test. The killable `top_k - 1 -> top_k + 1`
  # mutant is covered by the len == top_k + 1 boundary tests instead.
  --exclude-re "replace - with / in merge_"
  --exclude-re "replace > with >= in merge_"
  # CLI entry points: async I/O + side effects; --lib mutation cannot kill
  # "replace run_query -> Result<()> with Ok(())" without integration fixtures.
  --exclude-re "run_query"
  --exclude-re "replace run_query"
  # run_watch is the same class: a CLI entry point whose loop only exits on
  # Ctrl+C or on the event channel closing (the framework owns the sender, so
  # the latter is unreachable while the function holds it). Neither exit is
  # reachable from --lib, so "replace run_watch -> Result<()> with Ok(())" is
  # unkillable without a signal-sending integration fixture. Residual: the
  # call-site wiring (shutdown().await on the watch exit path) is covered by
  # review and a manual SIGINT smoke, not by a gate; the McpHandler and
  # framework_cleanup tests cover shutdown() itself, not its callers.
  --exclude-re "run_watch"
  # run_inject is the same class: a CLI command body reached only from the
  # binary's match arm (src/bin/csm.rs:141), so under --lib nothing calls it and
  # "replace run_inject -> Result<()> with Ok(())" cannot be killed. This mutant
  # appeared here only because #817 changed one line of the file (src/cli/commands/
  # inject.rs:22 now passes ttl_cleanup_interval_seconds = 0), not because inject's
  # behaviour got weaker: the flag itself is pinned by the parse tests in
  # src/cli/args_commands.rs.
  # Residual, stated plainly: the exclusion buys no coverage for the entry point
  # body. `csm inject` exit statuses are covered by tests/cli_integration.rs, which
  # is the *integration* profile — this job deliberately runs --lib only.
  --exclude-re "run_inject"
  # mcp::serve is the run_watch class: it is `serve_with_shutdown(config,
  # operator_shutdown())` (src/mcp/server.rs:72/97), so the body's work only
  # becomes observable in a process that receives SIGINT/SIGTERM — impossible
  # from `--lib`, where there is no operator signal and no transport to end.
  # Residual, stated plainly: this exclusion buys no coverage for the entry-point
  # body. The hop is pinned elsewhere — `src/mcp/server_tests.rs::
  # sse_exit_stops_the_ttl_cleanup_task` asserts the effect through
  # `serve_with_handler` (live reaper slot -> signal -> await -> slot empty),
  # `serve_with_shutdown_returns_when_the_signal_resolves` now requires the
  # listener to accept a connection before signalling, and the process-level
  # exits are covered by tests/cli_shutdown_signal.rs::
  # serve_sse_stops_cooperatively_on_sig{term,int} (integration profile). The
  # pattern is anchored on "replace serve ->" rather than "serve" so it cannot
  # swallow a `serve_*` helper `--lib` can test: `serve_with_handler` and
  # `run_sse_server_with_shutdown` are CAUGHT in the 2026-10-15 baseline.
  --exclude-re "replace serve ->"
  # src/mcp/tools.rs `| -> ^` in parse_hvec_u64_halves (2026-10-15, #833): the
  # operands are `u128::from(high) << 64` (bits 64..128) and `u128::from(low)`
  # (bits 0..64), disjoint by construction, so `|` and `^` agree on every input —
  # an equivalent mutant, not a weak test. Measured in the same baseline: the
  # sibling `| -> &` and all seven shift/arithmetic mutants in that function are
  # CAUGHT by tools_tests.rs::test_parse_hvec_legacy_u64_halves.
  --exclude-re "replace \| with \^ in parse_hvec_u64_halves"
  # src/bin/csm.rs: tracing setup, error formatting, shell completion, and main
  # are CLI-only concerns (side-effectful, process-exit, I/O); untestable via --lib.
  --exclude "src/bin/csm.rs"
  # merge_with > -> >=: equivalent mutant — equal scores produce same result
  # for both operators (keeping existing value vs overwriting with same value).
  --exclude-re "replace > with >= in AbsenceEntry::merge_with"
  # equivalent mutants in normalize_scores and merge_results (replacing < with <=, > with >= does not alter correctness)
  --exclude-re "replace < with <= in .*normalize_scores"
  --exclude-re "replace > with >= in .*normalize_scores"
  --exclude-re "replace < with <= in .*merge_results"
  --exclude-re "replace > with >= in .*merge_results"
  # query || -> &&: equivalent mutant — with top_k=0, find_similar+truncate(0)
  # also returns empty; with empty ns, find_similar returns empty too.
  --exclude-re "replace \|\| with && in BridgeRetrieval::query"
  # confidence sum/len: /->% and /->* are unkillable without knowing exact scores
  # at test time; the weight arithmetic is already covered by compute_final_score tests.
  --exclude-re "replace / with % in BridgeRetrieval::compile_packet"
  --exclude-re "replace / with \* in BridgeRetrieval::compile_packet"
  # unix_now_secs (wasm stub) and framework latency metrics: I/O and side-effectful
  # timing paths; not exercisable deterministically under --lib.
  --exclude "src/export_payload.rs"
  # framework Drop, builder, and latency-metric arithmetic: side-effectful async paths
  # observable only via integration tests (not --lib).
  --exclude-re "replace <impl Drop for ChaoticSemanticFramework>::drop"
  --exclude-re "replace ChaoticSemanticFramework::builder"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::inject_concept"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::inject_concept_with_metadata"
  --exclude-re "replace <= with > in ChaoticSemanticFramework::probe"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::probe"
  --exclude-re "replace >= with < in ChaoticSemanticFramework::probe"
  # src/bridge_persistence.rs: every Persistence:: method gates on
  # acquire_remote_slot().await? (a real network/DB semaphore). Under --lib there are no
  # integration fixtures to satisfy it, so these mutants cannot be killed deterministically
  # (they hang into a test timeout or survive unobserved). Persistence correctness is covered
  # by integration tests, not --lib mutation — same rationale as src/persistence_wasm.rs and
  # src/export_payload.rs. This file-level exclude also covers AbsenceEntry::normalize,
  # so no separate mutant-label exclude is needed.
  --exclude "src/bridge_persistence.rs"
  # M1 BM25 absence short-circuit (ADR-0094 follow-up): absence_short_circuit
  # is a framework async path that requires a persisted absence store and a
  # namespace revision read; the decision itself (lookup_absence) is covered by
  # --lib unit tests, while the wrapper's abstention shape is covered by
  # tests/bm25_absence_short_circuit.rs, which the --lib mutation profile does
  # not run (same rationale as the framework probe excludes above).
  --exclude-re "absence_short_circuit"
  # Test scaffolding: StubStore::list_absences is a trait method required by the
  # AbsenceStore stub but never called by the unit test, so mutating it to
  # Ok(vec![]) is unobservable.
  --exclude-re "StubStore>::list_absences"
  # Feature-cfg'd code is enumerated but never compiled under this profile, so its
  # mutants are phantom: the mutation diff cannot change a binary that does not
  # contain the code, every test run passes, and the mutant is reported "missed"
  # while no test could ever kill it. Measured on 4180c6c8 (run 40855342249): 19 of
  # the 73 missed mutants were this class. Proof, not inference — the error literal
  # "OTLP batch sizes and timeouts must be positive" from
  # src/observability/mod.rs:88 appears 0 times in
  # target/debug/deps/chaotic_semantic_memory-*.d293 built with exactly the FAST_ARGS
  # features below (control: "decay" appears 174 times in the same binary).
  # src/observability/mod.rs:13 is
  #   #![cfg(any(feature = "prometheus", feature = "otlp-json", feature = "otlp"))]
  # and the fast profile enables `default = cli,persistence,parallel` + `mcp` only, so
  # the whole module tree is out — including prom.rs (`#[cfg(feature = "prometheus")]`
  # at mod.rs:22). Remove these two excludes when #952 makes generation cfg-aware or
  # when the profile gains an observability feature; until then they are the only way
  # the score reflects testable code.
  --exclude "src/observability/*"
  # Same class, one function: extract_trace_parent and its
  # opentelemetry::propagation::Extractor impl (src/mcp/server.rs:348-360) sit behind
  # #[cfg(all(feature = "otlp", not(target_arch = "wasm32")))]; `otlp` is opt-in and
  # off here. Proven the same way — "traceparent was not applied to the request span"
  # occurs 0 times in the built lib, while its ungated neighbour require_bearer's
  # "Unauthorized" occurs 3 times, so server.rs itself is in the build and only these
  # mutants are excluded. The trace-context wiring is covered by
  # tests/mcp_streamable_http_integration.rs and src/mcp/server_tests.rs (which is a
  # #[cfg(test)] module of the ungated half of server.rs), not by --lib mutation here.
  --exclude-re "extract_trace_parent"
  # Issue #952 retired the four wasm32 clock-pair exclusions that used to live
  # here. The `Date::now() - start` subtraction sat under
  # #[cfg(target_arch = "wasm32")] next to a native Instant sibling, so the
  # native --lib profile never compiled it and the mutants were MISSED forever.
  # `crate::clock::Clock` now owns the one cfg-split source and the subtraction
  # is native-compiled (`elapsed_ms`), so no exclusion is needed; keeping one
  # would hide a real mutant in the same function.
  # Fourth of the same rule. `hybrid_result_to_array` (src/wasm_ext.rs:34-35) is under
  # #[cfg(target_arch = "wasm32")] like the two operators above, and its two
  # `-> Result<Array, JsValue> with Ok(Default::default())` mutants were both MISSED on
  # run 40855342249. Proof on the binary this profile actually builds (cargo test
  # --no-run --lib -p chaotic_semantic_memory --features chaotic_semantic_memory/mcp,
  # rustc as pinned): "failed to set JS property" (wasm_ext.rs:43, inside the gated fn)
  # occurs 0 times, while "min_strength must be finite" (wasm_ext.rs:26, the ungated
  # helper in the same file) occurs once — so the file is linked and only this fn is
  # absent. Named by function, not by file, because src/wasm_ext.rs is not wholly gated;
  # its only callers (wasm_ext.rs:195, :206) are WasmFramework methods that the
  # pre-existing --exclude-re "WasmFramework::" already covers.
  --exclude-re "hybrid_result_to_array"
)

# Arm split: cargo-mutants runs one `cargo test --no-run --package=X` per selected
# package, and a package-qualified `--features pkg/feat` is invalid in the command
# built for any other package. So `chaotic_semantic_memory/mcp` can only be passed
# when the root package stands alone. A package whose own code sits behind its own
# features needs an arm of its own for the same reason: an unqualified
# `--features chaotic-hashing` is invalid in `cargo test --package=csm-retrieval`,
# and `--all-features` cannot be scoped to one package inside a multi-package
# selection. Three kinds of arm exist in the fast profile: `root` (mcp, because
# `src/mcp/**` is `cfg(feature = "mcp")`), one `feat-<pkg>` arm per entry of
# MUTATION_FEATURE_ARMS, and `crates` (everything else, no features). The nightly
# `full` profile keeps its single historical arm.
#
# Why `--all-features` rather than the one gating feature: that is what the crate's
# own CI build already uses (`ci.yml:373` runs `cargo test -p csm-chaos
# --all-features`, and its comment says it is "the one place they are compiled and
# tested"), so the mutation job was the only build that still could not see the
# module. Hand-picking breaks too: `--features experimental-voronoi-lsh` alone
# leaves csm-chaos `no_std` (`#![cfg_attr(not(feature = "std"), no_std)]`,
# `default = []`), and the module's own unit tests then fail to compile on `vec!`.
# Measured on the #998 diff with cargo-mutants 27.1.0, `-- --lib`:
#   -p csm-chaos                    -> 12 caught, 1 unviable, 3 MISSED
#   -p csm-chaos --all-features     -> 14 caught, 2 unviable, 0 MISSED
#   -p csm-core-lib                 -> 3 MISSED
#   -p csm-core-lib --all-features  -> 2 caught, 1 unviable, 2 MISSED
# `voronoi_lsh` is `#[cfg(feature = "experimental-voronoi-lsh")]`
# (`crates/csm-chaos/src/hashing/mod.rs:2`) and the whole `chaotic_lsh` wrapper is
# `#![cfg(feature = "chaotic-hashing")]`; cargo-mutants lists mutants from source
# text, so it kept generating them, built a program without them, passed every
# test, and called the result MISSED. The last row's 2 are not blindness —
# csm-core-lib's tests never read the wrapper's `input_dim()`, which #998's
# `71f04548` fixes. The score is caught/(total-unviable) summed over arms, so
# enabling features can only add killable mutants and move unbuildable ones out of
# the denominator; it cannot lower the bar by hiding a miss.
declare -A MUTATION_FEATURE_ARMS=(
  [csm-chaos]="--all-features"
  [csm-core-lib]="--all-features"
)

ROOT_SELECTED=false
CRATE_PKGS=()
FEATURE_ARM_PKGS=()
for mutation_pkg in "${MUTATION_PACKAGES[@]}"; do
  if [[ "${mutation_pkg}" == "${ROOT_PACKAGE}" ]]; then
    ROOT_SELECTED=true
  elif [[ -n "${MUTATION_FEATURE_ARMS[${mutation_pkg}]:-}" ]]; then
    FEATURE_ARM_PKGS+=("${mutation_pkg}")
  else
    CRATE_PKGS+=("${mutation_pkg}")
  fi
done
ARM_NAMES=()
if [[ "${PROFILE}" == "full" ]]; then
  ARM_NAMES=(full)
else
  if [[ "${ROOT_SELECTED}" == "true" ]]; then
    ARM_NAMES+=(root)
  fi
  if [[ ${#CRATE_PKGS[@]} -gt 0 ]]; then
    ARM_NAMES+=(crates)
  fi
  if [[ ${#FEATURE_ARM_PKGS[@]} -gt 0 ]]; then
    feat_arm_pkg=""
    for feat_arm_pkg in "${FEATURE_ARM_PKGS[@]}"; do
      ARM_NAMES+=("feat-${feat_arm_pkg}")
    done
  fi
fi

# Populates ARM_PKGS (flat `-p` args) and ARM_FEATURE_ARGS (the arm's feature flags)
# for one arm.
configure_arm() {
  ARM_PKGS=()
  ARM_FEATURE_ARGS=()
  case "$1" in
    root)
      ARM_PKGS=(-p "${ROOT_PACKAGE}")
      if [[ -n "${MCP_FEATURE:-}" ]]; then
        ARM_FEATURE_ARGS=(--features "${MCP_FEATURE}")
      fi
      ;;
    crates)
      local crate_pkg
      for crate_pkg in "${CRATE_PKGS[@]}"; do
        ARM_PKGS+=(-p "${crate_pkg}")
      done
      ;;
    feat-*)
      local feat_pkg="${1#feat-}"
      local -a feat_args=()
      read -r -a feat_args <<< "${MUTATION_FEATURE_ARMS[${feat_pkg}]}"
      ARM_PKGS=(-p "${feat_pkg}")
      ARM_FEATURE_ARGS=("${feat_args[@]}")
      ;;
    full) ;;
  esac
}

# Preflight count (omit -j; listing does not need parallel workers).
#
# This probe decides whether the CI check runs at all, so it must not conflate
# three different things the old version reported identically as "no mutants":
#   1. the probe itself failed (cargo-mutants error, unparseable diff, bad scope)
#      -> a broken gate: exit 1, always;
#   2. the diff selects production Rust but no *changed line* carries a mutant
#      (comment-only edits, `use` lines, cfg-gated code, a return type cargo-mutants
#      cannot synthesise a value for) -> no evidence exists to gather: loud warning,
#      exit 0. Failing here red-gates legitimate PRs forever, which is what blocked
#      PR #977 before the scope fix above made its mutants visible;
#   3. the diff selects no production Rust at all -> nothing this job can say.
# stderr and the exit status are both captured: `2>/dev/null || true` hid case 1
# inside case 2's message, and the gate's own report then claimed "no mutants".
# Case 2 is NOT a passing mutation result and is written as such. The residual hole
# (a diff whose lines all sit inside a cfg-disabled block reads as case 2 while real
# risk exists) is issue #952's inventory check, not something this probe can see.
declare -A ARM_COUNTS=()
MUTANT_COUNT=0
if [[ "${PROFILE}" == "fast" ]] && [[ ${#FAST_ARGS[@]} -gt 0 ]]; then
  : >"${PREFLIGHT_LOG}"
  for arm in "${ARM_NAMES[@]}"; do
    configure_arm "${arm}"
    ARM_ARGS=("${FAST_ARGS[@]}" "${ARM_PKGS[@]}")
    if [[ ${#ARM_FEATURE_ARGS[@]} -gt 0 ]]; then
      ARM_ARGS+=("${ARM_FEATURE_ARGS[@]}")
    fi
    # Name it `…-<arm>-latest.log`, not `…latest.log.<arm>`: .gitignore:46 ignores
    # `/progress/mutation/*.log`, and a suffix-anchored glob does not match a file
    # that ends in the arm name. Measured — the dotted form left
    # `fast-preflight-latest.log.root` and `.crates` as untracked residue in
    # `git status`, which Phase 1 step 2 reads as a dirty tree.
    ARM_PREFLIGHT="${OUT_DIR}/${PROFILE}-preflight-${arm}-latest.log"
    LIST_RC=0
    cargo mutants --list "${ARM_ARGS[@]}" "${EXCLUDE_ARGS[@]}" >"${ARM_PREFLIGHT}" 2>&1 || LIST_RC=$?
    if [[ "${LIST_RC}" -ne 0 ]]; then
      echo "error: mutation preflight failed in arm '${arm}' (exit ${LIST_RC}); no mutation evidence was produced" >&2
      sed 's/^/  /' "${ARM_PREFLIGHT}" >&2
      {
        echo "# Unit-Target Mutation Test Report"
        echo
        echo "- Timestamp (UTC): ${TIMESTAMP}"
        echo "- Profile: ${PROFILE}"
        echo "- Arms: ${ARM_NAMES[*]}"
        echo "- Mutants: unknown (arm '${arm}' preflight failed with exit ${LIST_RC})"
        echo "- Result: not run (preflight failure; not a passing mutation result)"
      } >"${REPORT_FILE}"
      exit 1
    fi
    ARM_COUNTS["${arm}"]="$(grep -cE '\.rs:' "${ARM_PREFLIGHT}" || true)"
    MUTANT_COUNT=$((MUTANT_COUNT + ${ARM_COUNTS["${arm}"]:-0}))
    cat "${ARM_PREFLIGHT}" >>"${PREFLIGHT_LOG}"
    echo "mutation preflight: arm '${arm}' (${ARM_PKGS[*]:-workspace default}) selected ${ARM_COUNTS[$arm]} mutants" >&2
  done
  echo "mutation preflight: ${MUTANT_COUNT} mutants to test (jobs=${JOBS}, packages=${MUTATION_PACKAGES[*]})" >&2
  if [[ "${MUTANT_COUNT}" -eq 0 ]]; then
    echo "No mutants generated for the bounded diff; no mutation evidence was produced" | tee "${LOG_FILE}"
    {
      echo "# Unit-Target Mutation Test Report"
      echo
      echo "- Timestamp (UTC): ${TIMESTAMP}"
      echo "- Profile: ${PROFILE}"
      echo "- Packages inspected: ${MUTATION_PACKAGES[*]}"
      echo "- Mutants: 0"
      if [[ "${DIFF_RUST_SELECTED}" == "true" ]]; then
        echo "- Result: not run (diff selects production Rust but no changed line is mutation-visible; NOT A PASS)"
      else
        echo "- Result: not run (no production Rust source selected by the diff; NOT A PASS)"
      fi
    } >"${REPORT_FILE}"
    if [[ "${CI_MODE}" == "true" && "${DIFF_RUST_SELECTED}" == "true" ]]; then
      echo "::warning title=Mutation gate::0 mutation-visible lines in the changed Rust sources. Packages inspected: ${MUTATION_PACKAGES[*]}. No mutant was tested — this is NOT a passing mutation result."
      echo "warning: CI mutation check selected production Rust but no changed line carries a mutant; the gate reports no evidence instead of failing the PR" >&2
    fi
    if [[ -n "${SHARD}" ]]; then
      # Completeness marker for the aggregator: this shard ran and its slice was
      # empty by construction, not because the job died.
      echo "${SHARD}" > "${OUT_DIR}/no-mutants"
    fi
    exit 0
  fi
fi

set -o pipefail
set +e # cargo-mutants exits 2 when any mutant is missed; we evaluate the score below
: >"${LOG_FILE}"
MARKER="$(mktemp)"
trap 'rm -f "${MARKER}" "${TMP_PKGS}" "${DIFF_FILE:-}"' EXIT
ARM_RESULTS=()
RESULT=0
for arm in "${ARM_NAMES[@]}"; do
  configure_arm "${arm}"
  # An arm whose diff-selected count is 0 cannot produce evidence, and running it
  # would still pay a full baseline build. Only the fast profile has per-arm counts
  # (the nightly sweep has no preflight), so skip only there.
  if [[ "${PROFILE}" != "full" && "${ARM_COUNTS["${arm}"]:-0}" -eq 0 ]]; then
    echo "mutation arm '${arm}': 0 selected mutants, skipped" | tee -a "${LOG_FILE}" >&2
    ARM_RESULTS+=("${arm}=skipped")
    continue
  fi
  ARM_ARGS=("${MUTANTS_ARGS[@]}" "${ARM_PKGS[@]}")
  if [[ ${#ARM_FEATURE_ARGS[@]} -gt 0 ]]; then
    ARM_ARGS+=("${ARM_FEATURE_ARGS[@]}")
  fi
  ARM_OUT_DIR="${OUT_DIR}/${PROFILE}-${TIMESTAMP}-${arm}"
  ARM_LOG="${ARM_OUT_DIR}.log"
  mkdir -p "${ARM_OUT_DIR}"
  RUN_CMD=(cargo mutants "${ARM_ARGS[@]}" "${EXCLUDE_ARGS[@]}" "$@")
  if [[ ${#TEST_ARGS[@]} -gt 0 ]]; then
    RUN_CMD+=(-- "${TEST_ARGS[@]}")
  fi
  {
    echo ""
    echo "===== mutation arm: ${arm} (packages: ${ARM_PKGS[*]:-workspace default}, features: ${ARM_FEATURE_ARGS[*]:-none}) ====="
    printf '%q ' "${RUN_CMD[@]}"
    echo ""
  } | tee -a "${LOG_FILE}"
  touch "${MARKER}"
  RUSTFLAGS="" RUST_BACKTRACE=0 "${RUN_CMD[@]}" >>"${ARM_LOG}" 2>&1
  arm_rc=$?
  cat "${ARM_LOG}" >>"${LOG_FILE}"
  # `mutants.out` is written relative to the workspace root; `-newer "${MARKER}"`
  # distinguishes this arm's report from a stale one in the tree, and the copy keeps
  # the arms from overwriting each other. `mutants.out/` is gitignored at any depth,
  # so nothing here can leave the working tree dirty.
  if [[ -f mutants.out/outcomes.json ]] && [[ -n "$(find mutants.out/outcomes.json -newer "${MARKER}" 2>/dev/null)" ]]; then
    rm -rf "${ARM_OUT_DIR}/mutants.out"
    cp -a mutants.out "${ARM_OUT_DIR}/mutants.out"
    echo "mutation arm '${arm}' report copied to ${ARM_OUT_DIR}/mutants.out" >&2
  fi
  RESULT="${arm_rc}"
  ARM_RESULTS+=("${arm}=${arm_rc}")
done
set +o pipefail
set -e

# Ship cargo-mutants' own report with the CI artifact. The tee'd stdout only carries what
# cargo-mutants chose to print — caught mutants are silent, so a run that finishes 31 of 224
# before the ceiling shows 5 lines and looks stalled (that misread cost a wrong diagnosis
# here) — and `mutants.out/outcomes.json` is the record that answers it: per-mutant Build/Test
# phase durations, with `mutants.out/debug.log` carrying the timestamps that separate "this
# mutant is slow" from "the gap between mutants is slow". Outcomes are written as the sweep
# progresses, so a mid-run cancellation still leaves the partial data. Each arm now goes to
# its own `--output` directory under OUT_DIR (uploaded as `path: progress/mutation`), which
# also keeps arms from overwriting one another's outcomes; scripts/mutation_inventory.py reads
# `mutants.out/outcomes.json` relative to its CWD, so it runs once per arm from that arm's
# directory rather than from the workspace root.

{
  echo "# Unit-Target Mutation Test Report"
  echo
  echo "- Timestamp (UTC): ${TIMESTAMP}"
  echo "- Profile: ${PROFILE}"
  if [[ "${PROFILE}" == "fast" ]]; then
    echo "- Target Set: lib target only (--lib); the package scope lives on each arm, not the test command"
    echo "- Arms: ${ARM_RESULTS[*]}"
    # `${ARM_COUNTS[*]}` walks a hash map, so it does not follow `${ARM_NAMES[*]}`:
    # with three arms the line read `root crates feat-csm-core-lib = 0 1 0` while the
    # real counts were 1 0 0 — a report that sends a reader to the wrong arm. Built
    # in ARM_NAMES order so the labels and the numbers describe the same arms.
    PREFLIGHT_COUNTS=""
    for arm in "${ARM_NAMES[@]}"; do
      PREFLIGHT_COUNTS+="${PREFLIGHT_COUNTS:+ }${ARM_COUNTS[${arm}]:-n/a}"
    done
    echo "- Preflight: ${ARM_NAMES[*]} = ${PREFLIGHT_COUNTS}"
  else
    echo "- Target Set: full cargo test suite; discovery is the root package (see the full-profile note above)"
  fi
  echo "- Exit code: ${RESULT}"
  echo "- Jobs: ${JOBS}"
  echo "- Test args: ${TEST_ARGS[*]:-(full suite)}"
  echo "- Log: \`${LOG_FILE}\`"
  echo
  echo "## Tail"
  echo
  echo '```text'
  tail -n 40 "${LOG_FILE}"
  echo '```'
} >"${REPORT_FILE}"

for arm in "${ARM_NAMES[@]}"; do
  ARM_OUT_DIR="${OUT_DIR}/${PROFILE}-${TIMESTAMP}-${arm}"
  if [[ -f "${ARM_OUT_DIR}/mutants.out/outcomes.json" ]]; then
    {
      echo
      echo "## Arm \`${arm}\` mutation inventory"
      echo
    } >>"${REPORT_FILE}"
    (cd "${ARM_OUT_DIR}" && python3 "${SCRIPT_DIR}/mutation_inventory.py") >>"${REPORT_FILE}" \
      || echo "arm '${arm}' inventory could not be rendered" >>"${REPORT_FILE}"
  fi
done

echo "wrote ${REPORT_FILE}"

if [[ -n "${SHARD}" ]]; then
  echo "mutation shard ${SHARD}: evidence written; the mutation-test aggregator owns the union verdict" >&2
  if [[ "${RESULT}" -eq 0 || "${RESULT}" -eq 2 ]]; then
    echo "${SHARD}" > "${OUT_DIR}/shard-complete"
    exit 0
  fi
  exit "${RESULT}"
fi

if [[ "${CI_MODE}" == "true" ]]; then
  SCORE=""
  # One summary line per arm that ran. The gate's number is the UNION of the arms —
  # summed caught over summed viable, never an average of per-arm percentages: an
  # average would let an arm that killed nothing pass on its sibling's score, which
  # is the same arithmetic issue #951 demands across shards.
  SUMMARY_LINES="$(grep -cE '[0-9]+ mutant[s]? tested in .*' "${LOG_FILE}" || true)"
  if [[ "${SUMMARY_LINES:-0}" -gt 0 ]]; then
    read -r TOTAL CAUGHT TIMEOUTS MISSED UNVIABLE < <(
      grep -oE '[0-9]+ mutant[s]? tested in .*' "${LOG_FILE}" | awk '
        function num(re,   m) {
          if (match($0, re)) { m = substr($0, RSTART, RLENGTH); gsub(/[^0-9]/, "", m); return m + 0 }
          return 0
        }
        { t += $1; c += num("[0-9]+ caught"); to += num("[0-9]+ timeout"); mi += num("[0-9]+ missed"); un += num("[0-9]+ unviable") }
        END { printf "%d %d %d %d %d\n", t, c, to, mi, un }
      '
    )
    VIABLE=$((TOTAL - UNVIABLE))
    # ADR-0095: timeouts count as unresolved (not detected), not caught.
    # Infinite-loop mutants are a quality issue, not a detection success.
    EFFECTIVE_CAUGHT=$((CAUGHT))
    UNRESOLVED=$((MISSED + TIMEOUTS))
    if [[ "${VIABLE}" -gt 0 ]]; then
      SCORE="$(awk -v c="${EFFECTIVE_CAUGHT}" -v v="${VIABLE}" 'BEGIN { printf "%.4f", c*100/v }')"
    else
      SCORE="100"
    fi
    # Publish inventory
    echo "unit-target mutation inventory: caught=${CAUGHT} missed=${MISSED} timeout=${TIMEOUTS} unviable=${UNVIABLE} unresolved=${UNRESOLVED}" >&2
    echo "unit-target mutation summary: total=${TOTAL} caught=${CAUGHT} timeout=${TIMEOUTS} missed=${MISSED} unviable=${UNVIABLE} score=${SCORE}%" >&2
  fi
  if [[ -z "${SCORE}" ]]; then
    SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
  fi

  # Timeout mutants remain unresolved and are excluded from caught outcomes; the
  # budget below prevents hangs from silently weakening the score.
  # Override: MUTATION_TIMEOUT_BUDGET (absolute), MUTATION_TIMEOUT_FRACTION (of viable).
  TIMEOUT_BUDGET="${MUTATION_TIMEOUT_BUDGET:-5}"
  TIMEOUT_FRACTION="${MUTATION_TIMEOUT_FRACTION:-0.10}"
  if [[ -n "${TIMEOUTS:-}" ]]; then
    if [[ "${TIMEOUTS}" -gt "${TIMEOUT_BUDGET}" ]]; then
      echo "error: ${TIMEOUTS} mutation timeouts exceed absolute budget ${TIMEOUT_BUDGET}" >&2
      exit 1
    fi
    if [[ -n "${VIABLE:-}" && "${VIABLE}" -ge 10 ]]; then
      FRAC_LIMIT="$(awk -v v="${VIABLE}" -v f="${TIMEOUT_FRACTION}" 'BEGIN { n=v*f; printf "%d", (n==int(n)?n:int(n)+1) }')"
      if [[ "${TIMEOUTS}" -gt "${FRAC_LIMIT}" ]]; then
        echo "error: ${TIMEOUTS} mutation timeouts exceed ${TIMEOUT_FRACTION} of viable=${VIABLE} (limit ${FRAC_LIMIT})" >&2
        exit 1
      fi
    fi
    if [[ "${TIMEOUTS}" -gt 0 ]]; then
      echo "mutation timeouts: ${TIMEOUTS} (budget abs=${TIMEOUT_BUDGET})" >&2
    fi
  fi

  if [[ "${SCORE}" == "0" ]]; then
    echo "error: mutation run generated no scoreable mutants; result is not a pass" >&2
    exit 1
  elif awk -v s="${SCORE}" -v t="${THRESHOLD}" 'BEGIN { exit !(s >= t) }'; then
    echo "unit-target mutation score ${SCORE}% >= ${THRESHOLD}%, CI check passed"
    exit 0
  else
    echo "unit-target mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
    exit 1
  fi
fi

exit "${RESULT}"
