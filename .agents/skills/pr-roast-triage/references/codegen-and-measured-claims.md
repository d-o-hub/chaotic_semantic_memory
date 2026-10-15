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
