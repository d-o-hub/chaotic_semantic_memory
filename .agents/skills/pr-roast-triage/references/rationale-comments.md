# Rationale-comment rubric detail (PR #767 family, 2026-09-25)

Extracted from `SKILL.md` on 2026-10-05 to respect the 250-LOC skill cap. Nothing here
was deleted; the parent rule stays in `SKILL.md` under
**Rationale comments: never deleted to "shorten"**.

- **Verify before accusing.** A 21-line doc removal *looked* like comment-stripping, but
  `grep -c "ADR-0094"` returned 3 → 3: the rationale was reworded, not lost. Count the
  knowledge that survives (`grep -c` the ADR/issue tag on both sides), then name the exact
  contract text that did not — here the `#[cfg(feature = "persistence")]` prose
  ("Only available when…", "no-op since persistence is unavailable") on ~7 methods.
  Rewording is a nit; losing a documented contract is the violation. Never report the
  former as the latter.
- **A surviving tag is not a surviving contract.** Check every removed sentence
  **against the code it documents**, not against the diff's line count. The same PR's
  rework kept all three `ADR-0094` mentions yet dropped two behaviour statements that are
  verifiable in the source: the pool-size clamp/default (`clamp(1, MAX)` and default 10)
  and "`build()` rejects the configuration with `UnsupportedOperation`". A tag tells a
  reader where to look; the sentence tells them what happens.
- **Don't restore for symmetry.** A "Only available when feature X" marker is redundant
  when `#[cfg(feature = "X")]` sits on the same item and rustdoc renders the gate. If you
  do restore it, commit as a maintainer action with per-line evidence (`4f8c7b1`) and
  check **every** feature configuration (default, `--features cli`,
  `--no-default-features`).
- A PR can violate this rule and still be **right on code** (#767's
  `with_chaos_strength` leaked `NaN`/`±∞`). Say both; name what blocks merge.
- **Flipped test expectations are contract changes.** A test asserting "negative strength
  fails" becoming "negative clamps" belongs in the body as a deliberate contract change,
  not filed as a test fix.
- **`const fn` → `fn` is a public API change**; say so even when in-repo callers are all
  runtime.
- **Dedup deletes prose too** (2026-10-03, #816): the removed copy's `with_ttl` doc was
  the only place stating expiry resolves in `build()`, not at setter time — the owner's
  doc did not say it. Diff a duplicate pair in both directions and port rationale before
  deleting either side.
