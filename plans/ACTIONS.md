# ACTIONS — Active GOAP Action Queue

> **Compacted 2026-11-03 (ADR-0097 pattern).** This file holds **active
> actions only** (`queued` | `in_progress` | `blocked` | `deferred`); completed
> entries are not re-added. A verbatim pre-compaction snapshot — including the
> long per-action "last completed" narratives — is archived at
> `plans/.archive/2026-11-03-historical/ACTIONS_2026_11_03.md`. Earlier history:
> `plans/.archive/2026-08-08-historical/`, `plans/.archive/2026-07-20-historical/`
> and git.
>
> More recent completions are recorded in `plans/GOAP_STATE.md`
> (`action_last_completed`, exactly once, last key) and in git.
>
> Hygiene: when an action completes, remove it here, set
> `action_last_completed` in `GOAP_STATE.md`, and let the next dated
> reconciliation snapshot the file. Do not re-add completed entries.

```yaml
  - name: verify_npm_attestations_in_the_release_report
    github_issue: 955
    preconditions:
      verify_release_collects_all_verdicts: true
    effects:
      npm_attestations_verified_and_retry_dropped: true
    cost: 2
    status: queued
    # Premise corrected 2026-10-30: this was queued as "0 of 11 npm versions have provenance", which
    # the registry refutes — 19 of 21 DO (15/16 and 4/5; only chaotic_semantic_memory@0.1.0 and
    # csm@0.3.4 lack it). The probe that produced the zero read `versions[*].attestations` instead of
    # `versions[*].dist.attestations`. Surviving work: assert `.dist.attestations.provenance` in
    # verify-release (positive-controlled against the known-attested 0.3.8 and the known-unattested
    # 0.3.4) and drop the `|| npm publish` retry. Do not implement the original options — "accept
    # token publishing, drop --provenance" would delete attestations that exist today.
    # Measured 2026-10-31: `.dist.attestations` (shape `{url, provenance}`, both packages, 0.3.8 and
    # 0.3.9) is served ONLY by the abbreviated packument — `Accept: application/vnd.npm.install-v1+json`.
    # `npm view <pkg> dist --json` reads the full packument, which returns shasum/tarball/fileCount/
    # integrity and no `attestations` key, so an assertion written against `npm view` reports "missing"
    # on an attested release. Same class as #972's frozen substitution: the tool you reach for is not
    # the endpoint the data lives on. `npm audit signatures` is the third option and the only one that
    # verifies an attestation rather than its presence.


  - name: record_and_gate_the_zip_bundled_assumption
    github_issue: 966
    preconditions: []
    effects:
      zip_bundled_assumption_gated: true
    cost: 2
    status: queued
    # Measured 2026-10-30: Dependabot alert #40 is `GHSA-m665-v7pm-7grj` / CVE-2026-115905, **critical**,
    # published 2026-10-28T21:09:50Z, `zip <= 9.0.4` patched only in 9.0.5 while `Cargo.lock:7083` holds
    # 6.0.0 as a **build**-dependency of `libduckdb-sys` (sparse index `"kind":"build"`, `req":"^6"` — a
    # classification `Cargo.lock` itself cannot express). Harmless today because
    # `libduckdb-sys/build.rs:144` gates the download/extract module under `#[cfg(not(feature = "bundled"))]`
    # and `:371-374` is the `archive.extract()` call RUSTSEC-2026-0369 names, while
    # `crates/csm-duckdb/Cargo.toml:22` enables `bundled`. The gate blindness is the finding: no RUSTSEC id
    # exists, so `cargo deny`/`cargo audit` cannot report it and `deny.toml` has nothing to ignore.
    # Fix: record the decision beside the existing "No RUSTSEC ID" prose entries, and fail a gate when
    # `bundled` disappears from csm-duckdb while RUSTSEC-2026-0367/0368/0369 stay ignored, so those three
    # ignore reasons cannot expire silently.

  - name: bound_chaotic_lsh_and_voronoi_lsh_input_dim
    github_issue: 961
    preconditions: []
    effects:
      chaos_input_dim_rejected_not_clamped: true
    cost: 3
    status: queued

  - name: shard_the_mutation_test_context
    github_issue: 951
    preconditions:
      host_subprocesses_isolated_from_nix_env: true
    effects:
      mutation_test_sharded_behind_one_context: true
    cost: 5
    status: queued

  - name: detect_new_cfg_disabled_mutants
    github_issue: 952
    preconditions:
      mutation_test_sharded_behind_one_context: true
    effects:
      new_cfg_disabled_mutants_detected_by_inventory: true
    cost: 3
    status: queued

  - name: make_mutation_threshold_draft_invariant
    github_issue: 959
    preconditions:
      new_cfg_disabled_mutants_detected_by_inventory: true
    effects:
      mutation_threshold_independent_of_draft_flag: true
    cost: 2
    status: queued
```

Dependency reading, so a later session does not re-derive it: #960 -> #954 -> #955 is the
release chain (a refusal that cannot name the failing gate job, a verifier that stops at the
first missing artifact, an attestation that silently degrades). #948 and #949 and #961 are
independent of everything. #951/#952/#959 are **strictly sequential**: all three edit
`.github/workflows/ci.yml` and `scripts/mutation_test.sh` in the same regions (job
`mutation-test`, the `--exclude` block, `MUTATION_THRESHOLD`), and parallel PRs there
guarantee a conflict and a bot resolution that reverts one of them (`AGENTS.md` Rule 10).
`repair_the_release_gate_race` (#953) is not queued — it landed as PR #962 and is recorded as
`action_last_completed` in the 2026-11-03 archive snapshot.
