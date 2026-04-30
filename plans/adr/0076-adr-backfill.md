# ADR-0076: ADR Backfill — Restore Decision Provenance

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`plans/ADR_REGISTRY.md` references roughly 40 ADRs by number (0024–0066). On disk:

- `docs/adr/`: 2 files (0064, 0065)
- `plans/adr/`: 9 files (0042, 0046, 0057-0063)

That leaves ~29 ADRs claimed in the registry that have **no on-disk source of truth**. Decision rationale for v0.1.x – v0.3.x lives only in:
- Commit messages
- Inline `# 2026-MM-DD: <note>` comments in `plans/GOAP_STATE.md`
- Wave handoff notes in `plans/handoffs/`

This is fragile: when a key change is questioned six months from now, there is no canonical "why".

## Decision Drivers

- Don't fabricate decisions — reconstruct from real evidence
- Keep ADRs short — ≤ 1 page each is fine
- Mark all backfilled ADRs `Accepted (backfill)` to flag origin
- Preserve dual-location convention (`docs/adr/` vs `plans/adr/`)
- LOC: each ADR file ≤ 250 lines

## Considered Options

1. **Backfill all missing ADRs** in one wave from registry + git log + handoffs
2. Backfill only on-demand when a decision is questioned
3. Mark unbacked-up ADRs as "implicit" and stop tracking

## Decision Outcome

Chosen: **Option 1**. The cost (one wave of writing) is bounded; the benefit (durable rationale) accrues forever.

## Implementation

### Inventory step

Generate a checklist:
```bash
grep -oE 'ADR-00[0-9]{2}' plans/ADR_REGISTRY.md | sort -u > /tmp/registry_adrs.txt
ls docs/adr/ plans/adr/ | grep -oE '00[0-9]{2}' | sort -u > /tmp/disk_adrs.txt
comm -23 /tmp/registry_adrs.txt /tmp/disk_adrs.txt > /tmp/missing_adrs.txt
```

Expected ~29 missing IDs.

### Backfill template (per ADR)

For each missing ADR, populate:

```markdown
# ADR-NNNN: [Title from registry]

## Status
Accepted (backfilled 2026-04-30 — original decision predates this document)

## Context
[Reconstructed from: commit SHA, GOAP_STATE comment, handoff note]

## Decision
[The change that was actually shipped, derived from current source]

## Consequences
[What is true today as a result]

## Sources
- Commit: abc1234
- GOAP_STATE.md: line N
- Handoff: plans/handoffs/WX_*.md
```

### Where to write

- ADRs 0024–0041 → `plans/adr/` (legacy location)
- ADRs 0042–0066 → `plans/adr/` (already half-populated there)
- Future ADRs (0067+) → `plans/adr/` (single canonical location)
- `docs/adr/` 0064–0065 → leave in place; cross-link from registry

### Audit additions

After backfill, add to `plans/ADR_REGISTRY.md`:
- File path next to each ID
- Status column (Proposed / Accepted / Backfilled / Superseded)
- Date

### Verification gate

A new check in `scripts/validate.sh`:
```bash
# Every ADR ID in registry must have a file.
# Normalize both sides to the bare 4-digit form (e.g., "0024") so `comm`
# compares identical strings — registry uses `ADR-NNNN`, files use `NNNN-…md`.
registry_ids=$(grep -oE 'ADR-[0-9]{4}' plans/ADR_REGISTRY.md \
                 | sed -E 's/^ADR-//' | sort -u)
disk_ids=$(ls docs/adr plans/adr 2>/dev/null \
             | grep -oE '^[0-9]{4}' | sort -u)
missing=$(comm -23 <(echo "$registry_ids") <(echo "$disk_ids"))
if [ -n "$missing" ]; then
  echo "Missing ADR files for IDs:"
  echo "$missing" | sed 's/^/  /'
  exit 1
fi
```

**Note (Codex review feedback addressed 2026-04-30):** an earlier draft of this
snippet compared `ADR-NNNN` against `NNNN`, which would have reported every
registry entry as missing and broken the gate. The version above normalizes
both sides to the bare 4-digit form before diffing.

## Pros and Cons

### Pros
- Restores provenance for every documented decision
- Catches divergence between registry and reality
- Prevents future "where is ADR-0050?" confusion

### Cons
- One-shot writing cost (~29 short docs)
- Some reconstruction may be best-effort if commit messages are sparse
- New validation gate is one more thing to keep green

## Acceptance Criteria

- [ ] All registry-referenced ADR IDs have a file on disk
- [ ] Each backfilled ADR includes Sources section
- [ ] Registry updated with file paths and status
- [ ] `scripts/validate.sh` enforces parity
- [ ] No file > 250 lines
