# Rubric detail: zero-init/codegen claims, and measured claim pairs

Extracted from `SKILL.md` (250-LOC cap) on 2026-10-15, from #893 (a fifth
filing of the #800/#814 `permute` hunk plus the #783 `MaybeUninit` conversions)
and #889 (a re-file of #776 with a false before/after pair).

## A zero-init claim is a codegen claim — transcribe both bodies and disassemble

"Replaces zero-initialized 1 280-byte arrays with `MaybeUninit`" is falsifiable
in about a minute, without running the crate's own benchmarks:

```bash
mkdir -p /tmp/dse && cd /tmp/dse
# both bodies copied verbatim from origin/main and the PR head, identical
# signatures, each wrapped in `#[no_mangle] #[inline(never)]`
rustc -O -C codegen-units=1 -C debuginfo=0 dse.rs -o dse
nm -C --defined-only dse                    # identical address ⇒ byte-identical code
objdump -d --no-show-raw-insn dse | grep -nE 'memset|memcpy'
```

Measured on #893 (`rustc 1.88.0`, the version `rust-toolchain.toml` pins):

| body | symbols | disassembly |
|------|---------|-------------|
| `from_bytes` main vs PR (little-endian arm) | both at `0x14b50` | `memcpy` only — LLVM had already dead-store-eliminated the `[0u128; 80]` initializer, so there is no zero-fill to remove |
| `permute` aligned path, init before the branch | `0x14f20` | `memset` + `memcpy` |
| `permute` aligned path, init inside the branch (the PR's shape) | `0x14fd0` | `memset` + `memcpy` — **still there** |
| `permute` general path (`bit_shift != 0`, untouched by the PR) | `0x14cd0` | `memset` |

Two rules fall out of this:

1. **Moving a declaration into a branch is not a codegen fix.** LLVM cannot
   prove that `result[..80-w]` + `result[80-w..]` covers all 80 words for a
   runtime `w`, so it keeps the `memset` regardless of where the initializer
   sits. Removing it needs a shape the compiler can see through (one
   `copy_from_slice` into the destination, or two fixed-length
   `ptr::copy_nonoverlapping` calls), and then a disassembly showing it gone.
2. **The PR removed exactly one `memset`, on the path it short-circuits with
   `return *self`** — which is #800's change, measured on 2026-10-02 at
   81.7 → 57.3 ns on the direct call and 2 684 → 2 734 ns (noise) on the
   caller path. A codegen delta that exists only on the already-measured path
   is not new evidence; it inherits the old verdict.

Also recorded: on `rustc 1.88.0` the *old* bodies emit no `memset` for the
conversions, i.e. the #783 conclusion reproduces on the workspace's pinned
toolchain, not only on the 1.98.1 of the original inspection.

## Measure both arms yourself; a claimed "after" can be slower than the head

#889's body claimed *"~4.51 µs → ~675 ns (~85 % reduction)"*. Two worktrees at
`origin/main` and the PR head, one probe file copied into both, 200 000
iterations per arm, six interleaved rounds:

| round | `main` | head |
|-------|--------|------|
| 1 | 590.6 ns | 468.3 ns |
| 2 | 577.3 ns | 459.6 ns |
| 3 | 581.0 ns | 457.5 ns |
| 4 | 572.2 ns | 494.0 ns |

6/6 rounds same sign ⇒ ≈ −20 %, not −85 %. Both body numbers were wrong in
different directions: the claimed "after" (675 ns) is *slower* than the head's
own measured time (468 ns), and the claimed "before" is 7.6× the measured
`main` baseline. `4.51 µs` was not invented — it is #776's own claimed
**post-change** number for **3-token** input, carried into this body as the
"before". Cross-check a resubmission's numbers against the predecessor's roast
record before believing either.

Two more rules from the same PR:

- **A correctly parameterized bench does not fix a false body.** #889's added
  `encode_single_1tok` calls `encode("hello")` and *does* enter the new fast
  path (the opposite of #800's `permute(321)` trap), and the measured win is
  real — but the gate reads the body, so the repair is in the body (a
  `## Performance Evidence` section naming the bench id, the artifact and the
  before/after pair).
- **`Vec::with_capacity(n)` + `push` over a slice-backed iterator is still
  byte-identical to `collect()`** (#754 → #776 → #889, third filing).
  Re-measured here: 3-token encode is `2 allocs / 3 904 B` on *both* revisions,
  because `tokens.iter().enumerate().map(..)` is `ExactSizeIterator`-specialized.
  A counting `GlobalAlloc` settles it in one run; drop the hunk.

## An equivalence test on a fast path that is *identical* to the slow path kills no mutant

#889's fast path (`tokens.len() == 1 && ngram_size.is_none() → token_to_hvec`)
returns exactly what the general path returns for a one-token input:
`bundle(&[v])` returns `v` (`hyperdim.rs:118`) and `permute(0)` is the identity
(`hyperdim_tests.rs:48`). So its new test — `encode("hello") ==
token_to_hvec("hello")` — passes under the *slow* path too. Verified by applying
the mutant in a worktree:

```bash
# crates/csm-core-lib/src/encoder.rs: `== 1` → `!= 1`
cargo test --release -p csm-core-lib --lib encoder   # 10 passed, 0 failed
```

A test whose input cannot distinguish the two branches is a compile check with
an assertion attached. Kill that mutant with an input where the branches differ
(`encode("alpha beta gamma") != token_to_hvec("alpha")`), or with an allocation
count. Context: `scripts/mutation_test.sh:146` scores
`--lib -p csm-retrieval -p chaotic_semantic_memory`, so `csm-core-lib` is
outside the mutation gate today — an argument for the assertion, not against it.

## A dependency bump can break the MSRV pin with no visible cause

#887 (`uuid 1.26.1 → 1.28.0` in `/fuzz`) failed `Fuzz Workspace Build` with
`uuid@1.28.0 requires rustc 1.89.0`. crates.io metadata: `1.26.1` →
`rust_version 1.85.0`, `1.27.0` and `1.28.0` → **`1.89.0`**, while
`rust-toolchain.toml` pins `1.88.0` and the workspace declares
`rust-version = 1.88` — so the `--precise` downgrade the cargo error suggests
lands on the version already in the lock, and the intermediate release is not a
workaround either. Before treating a lockfile bump as trivial, check
`rust_version` for the target *and* the next version (crates.io
`/api/v1/crates/<crate>/<ver>`); the cure is a dependabot `ignore` with a
"remove when the MSRV moves" reason, or an MSRV move — a whole-repo decision.

## Interleaving on `csm-ref-01`: two binaries, not two rebuilds (2026-10-19, #917)

`csm-ref-01` is a ThinkPad T480 (i5-8350U, `powersave` governor) running a desktop session,
and a `cargo bench` leg competes with the rebuild that produced it. Absolute times for the
**same binary** moved ~50 % across one triage session as the load average fell from 3.7 to 2.3.

The failure mode is directional, not merely noisy. Toggling the source and rebuilding gives
legs that straddle a 2-minute build at different thermal states:

| leg | reading | note |
|---|---|---|
| with sort, first read | 190.29 µs | taken right after its own rebuild |
| without sort, first read | 255.69 µs | taken after a *different* rebuild |
| ⇒ naive conclusion | **sort is 25 % faster** | the claim appears confirmed |

Repeat the same two states as saved binaries and the sign inverts: A (no sort) 166.16 /
170.48 / 163.47 / 168.45 µs vs B (sort) 181.16 / 179.22 / 179.95 / 180.61 µs — every pair
positive, median 168.3 → 180.0, **+8 % slower**. Protocol:

1. Build arm A, `cp target/release/deps/<bench>-<hash> /tmp/bench_A`. Cargo reuses the same
   hashed filename for the variant — re-resolve with `ls -t` after each build, and confirm the
   toggle is actually in the source (`grep -c`) before trusting which binary is which.
2. Toggle, build arm B, `cp … /tmp/bench_B`.
3. Run `A B A B …` from `/tmp`, **never rebuilding between legs**, printing `/proc/loadavg`
   per leg so a reviewer can see the drift the pairing absorbed.
4. Run the bench binary directly with `--bench` (`/tmp/bench_A <filter> --bench --warm-up-time 1
   --measurement-time 3`); without `--bench` criterion executes in test mode and prints no
   timings, which looks like a silently passing run.
5. Quote the paired deltas and their consistency, not an absolute, and keep an unchanged-arm
   control (the canonical `bm25_search/search_1000_docs` A/B/A/B sign-flipped
   11.59 / 10.87 / 11.81 / 14.82 µs — that spread is the host's floor, so a claim smaller
   than it is unmeasurable here and needs the CI runner or a committed baseline).

For a change whose guard never fires on the repo's corpora (see the `bench-blindness`
reference), step 3's two arms differ only by the added code, so any residual delta is
codegen/layout — which is why #783's warning applies in reverse: a reproducible delta is
not confirmation of the stated mechanism, and the mechanism must be argued from the access
pattern. Here it predicted the measured sign: reordering the *term* loop cannot cluster
`doc_idx`, so the working set over `doc_scores`/`factors` is unchanged and the sort is pure
overhead.

## Search for the predecessor before you publish the verdict (PR #927, 2026-10-21)

I roasted #927 (`perf(encoder)`: guard ahead of `split_whitespace().collect()` on the single-token
path), posted it, and only *then* ran Step 3's symbol grep. The search was decisive on three axes and
cost two follow-up comments to correct the record:

1. **It refuted a charge I had asserted.** I wrote that "zero-allocation" was a misnomer because
   `HVec10240::new_seeded` "allocates 1 280 bytes". `pub struct HVec10240 { pub data: [u128; 80] }`
   (`hyperdim.rs:39`) is an inline array returned by value — stack, no heap. **Before calling a title
   a misnomer, check where the bytes live:** a `-> Self` of a fixed-size array is not an allocation.
   This is the mirror image of the existing rule that a by-value `-> Self` *rules out* a zero-copy claim.
2. **It upgraded "undeclared second change" to "already rejected twice".** The bundled
   `map(..).collect()` → `with_capacity` + push hunk is measured in this very file at
   `progress/LEARNINGS.md:159` (#776: both forms `1 alloc / n × 1 280 B` at n = 1/3/10, "**expect this
   hunk in most 'avoid intermediate allocations' bot PRs**"), was ordered dropped in
   `plans/PR_ROAST_2026_10_15.md` repair 1 (#889), and #897's commit message says it is
   "deliberately not included". A hunk with a recorded verdict **inherits it — do not re-litigate it,
   and do not ask for it back as a second PR.**
3. **It explained the magnitude before I measured anything.** #889 recorded the post-#897 profile as
   `1 alloc / 64 B`, "the surviving 64 B is the token `Vec<&str>`". So #927 could only ever remove one
   64-byte malloc/free — and 64 B is itself a mechanism check (a 1-element `Vec<&str>` takes minimum
   capacity 4 × 16 B). My measured −23.8 ns mean sat right at that ceiling.

Cheapest form, before composing: `grep -rn '<symbol>' plans/PR_ROAST_*.md progress/LEARNINGS.md`,
`git log --oneline -- <touched file>`, and **blame the lines the PR "adds"** — if `main` already
contains the change, the PR's delta is only what surrounds it.

## When a guard is behaviourally invisible, an allocation count is the only admissible evidence

A fast path that duplicates an existing early-return returns *identical values*, so no assertion can
distinguish presence from absence. #927's guard and `encoder.rs:241` are exactly this: #897's landed
tests (`encoder.rs:423`, `:438`) kill the `tokens.len() == 1` mutant and stay green whether the guard
exists or not. Demand the counting-`GlobalAlloc` before/after (`1 alloc / 64 B → 0 allocs`) instead of
a percentage, and check whether CI can even see the gap — `scripts/mutation_test.sh` scores `--lib` for
`csm-retrieval` + `chaotic_semantic_memory` only, so `csm-core-lib` sits outside the mutation gate.

## Report the arm mean *with* the unchanged-arm control, and read a sign count as the statistic

Absolute times on this host sit ~38 % above the committed `plans/evidence/bench_encoder_2026_10_15/`
values (`encode_single_1tok` ~650 ns here vs ~471 ns committed; `encode_short_3tok` +53 %), so
comparing against a committed baseline in absolute terms is invalid — only within-run pairing means
anything. #927 measured −23.8 ns mean over 6 pairs, which looks like a win until the control runs:
repeats of the **same binary** spread 3.4 ns and **41.1 ns** on that bench. The floor exceeded the
effect, so no single pair proved anything and a ~5 % headline is unmeasurable here.

Two rulings that survived, and why they differ:

- **6/6 pairs slower on `encode_batch_5docs` stands.** Sign consistency is not judged by delta size —
  six independent pairs in one direction is ≈1/64 under a random-sign null, and a wide floor cannot
  manufacture that. The mechanism was independent too: the guard scans `split_whitespace()` and the
  `collect()` below it scans again, so every multi-token input pays twice.
- **5/6 with one flipped pair does not establish a magnitude**, only a direction, and belongs in the
  verdict as "not resolvable on this instrument" rather than "overstated". Note the asymmetry:
  the *same* fixed saving is ~4 % against an inflated 650 ns base and ~7 % against a 471 ns base, so
  a host-dependent denominator can make a correct claim look inflated. Say which host class produced
  the number, because a reviewer cannot reproduce a pair measured on a different class.
