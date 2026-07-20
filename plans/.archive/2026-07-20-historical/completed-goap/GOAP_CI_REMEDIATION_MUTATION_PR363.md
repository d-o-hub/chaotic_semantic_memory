# GOAP: CI Failure Remediation — Mutation Gate (PR #363)

> bm25 perf PR ["perf(bm25): reduce allocations and HashMap lookups in
> search"](https://github.com/d-o-hub/chaotic_semantic_memory/pull/363),
> merged to `main` as `acee40b`.
> Analysis date: 2026-06-10. Failing run analyzed: `27261966916`
> (PR branch `perf/bm25-search-buffers-dedup-...`).
> Companion plan: this file. State recorded in `GOAP_STATE.md` and `ACTIONS.md`.

## 1. Goal State

```yaml
goal_state:
  mutation_test_job_passing: true          # CI "mutation-test" job green
  bm25_search_mutation_score_ge_85: true   # >= 85% threshold (scripts/mutation_test.sh)
  bm25_dedup_correctness_tested: true      # distinct query terms each contribute
```

## 2. Current State (observed)

```yaml
world_state:
  failing_job: mutation-test               # all other CI jobs SUCCESS
  failing_run: 27261966916
  cargo_mutants_version: 27.1.0            # CI; local repro used 27.0.0
  mutants_in_diff: 37                       # --in-diff scoped to PR #363 bm25.rs changes
  mutants_caught: 31
  mutants_missed: 6
  mutation_score_pct: 83.78                # 31/37 -> below 85 threshold -> FAIL
  threshold_pct: 85
```

CI invocation (`.github/workflows/ci.yml` → `scripts/mutation_test.sh fast --ci`):
- `--in-diff <git diff origin/main>` scopes mutants to the PR's changed lines.
- `MUTATION_THRESHOLD: 85`; score = `caught * 100 / total`.
- On a **push to `main`**, `base_ref` is empty so `DIFF_TARGET=origin/main` and
  the diff is empty → the script logs *"running full target set"* (no
  `--in-diff`). The per-PR gate (in-diff) is therefore the authoritative gate.

## 3. Root-Cause Analysis — the 6 missed mutants

All 6 live in `src/retrieval/bm25.rs`, in code introduced/restructured by
PR #363 (`Bm25Index::search` + the new `push_query_weight` helper).

| # | Location | Mutation | Verdict | Reason |
|---|----------|----------|---------|--------|
| 1 | `bm25.rs:271:31` | `query_tokens.len() <= 8` → `> 8` | **Equivalent** | Both dedup paths (linear scan vs `HashSet`) produce identical `query_weights`; flipping the threshold only swaps *which* algorithm runs. No observable output difference. |
| 2 | `bm25.rs:276:49` | `query_tokens[j] == term` → `!= term` | **KILLABLE** | Flipping marks every *distinct* term as a duplicate, so only the first query term is scored. Observable when each term maps to a different document. |
| 3 | `bm25.rs:341:33` | `scores.len() > top_k` → `>= top_k` | **Equivalent** | At `len == top_k` the extra `select_nth_unstable_by` + `truncate(top_k)` is a no-op; the trailing `sort_unstable_by` normalizes order. Identical output. |
| 4 | `bm25.rs:342:37` | `top_k - 1` → `top_k / 1` | **Equivalent** | Only reached when `len > top_k`. `nth = top_k` vs `top_k-1`; both are valid indices, `truncate(top_k)` then full sort yields the same top-k set. |
| 5 | `bm25.rs:366:19` | `df > 0` → `df >= 0` | **Equivalent** | `df` from `doc_freqs` is ≥ 1 for live terms. After `remove_document_at` (`df.saturating_sub(1)`) a term can reach `df == 0`, **but its postings list is emptied in the same pass** → the scoring loop iterates zero entries regardless of the guard. No observable difference. |
| 6 | `bm25.rs:369:24` | `idf > 0.0` → `idf >= 0.0` | **Equivalent** | `idf = ln((n+1)/(df+0.5))`. Since `df ≤ n` always, `(n+1)/(df+0.5) > 1` ⇒ `idf > 0` always. The two comparisons are identical for all reachable inputs. |

### Why the gate is fixable by killing exactly one mutant

`85%` of `37` is `31.45`, so the gate needs `≥ 32` caught.
Killing mutant #2 → `32/37 = 86.49% ≥ 85%` → **PASS**.
The remaining 5 are *equivalent mutants* (no test can distinguish them by
behavior; they are not bugs). They are documented here as accepted.

## 4. The Fix

Add two regression tests to `src/retrieval/bm25/tests.rs`:

1. `test_search_distinct_terms_each_contribute` — **kills mutant #2.**
   Indexes `doc_alpha` (only "alpha") and `doc_beta` (only "beta"), queries
   `["alpha","beta"]` (len ≤ 8 → linear-scan path), and asserts **both**
   documents are returned with positive scores. Under the `!=` mutant the
   second term is dropped and only one document is returned.
2. `test_search_dedup_hashset_path_distinct_terms` — coverage companion for
   the `> 8` token `HashSet` dedup branch; also asserts duplicate query terms
   do not inflate the score (`search(["a"])` == `search(["a","a"])`).

No production code changed — the BM25 logic is correct; the gap was missing
test coverage of the multi-distinct-term path.

## 5. Verification

```text
cargo test --lib retrieval::bm25        # 31 passed (was 29; +2)
cargo fmt --check                        # clean
cargo clippy --lib                       # clean (no warnings)
```

Mutant-kill proof (manual mutation, since local cargo-mutants hit an
unrelated `rust-lld` linker error in its sandbox build):

```text
# Apply mutant #2 manually:  bm25.rs:276  == term  ->  != term
cargo test ...test_search_distinct_terms_each_contribute
  -> FAILED: assertion `left == right` failed (left: 1, right: 2)
# Revert -> test passes again.
```

Result: with the new test, mutant #2 is caught → `32/37 = 86.49% ≥ 85%`.

## 6. Outcome

```yaml
result_state:
  mutation_test_job_passing: true          # 86.49% >= 85%
  bm25_dedup_correctness_tested: true
  equivalent_mutants_documented: 5         # mutants #1,#3,#4,#5,#6
  production_code_changed: false           # tests-only fix
  files_changed:
    - "src/retrieval/bm25/tests.rs (+2 tests, +66 LOC)"
    - "plans/GOAP_CI_REMEDIATION_MUTATION_PR363.md (this doc)"
    - "plans/GOAP_STATE.md (world_state)"
    - "plans/ACTIONS.md (action record)"
```

## 7. Follow-ups / Notes

- **Equivalent-mutant policy.** cargo-mutants cannot be made to "kill"
  equivalent mutants by adding tests. If future PRs re-touch these exact
  lines and the in-diff gate dips below 85%, the options are (in order of
  preference): (a) add a behavioral test if the mutant is actually killable;
  (b) refactor to remove the dead/equivalent comparison (cf. PR #346, which
  deleted an always-false branch); (c) as a last resort, scope an exclusion
  in `scripts/mutation_test.sh` (`--exclude-re`) with justification.
- **`df == 0` retention.** `remove_document_at` leaves `df == 0` entries in
  `doc_freqs` (only `saturating_sub`). Harmless today (postings emptied in
  sync), but a future cleanup could `remove` the key to keep the map tight.
