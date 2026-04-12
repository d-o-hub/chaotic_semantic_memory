# Handoff: Group F -> All (Wave 20 CI/Release Blockers)

## Tasks
- IQ-11 configure npm OIDC trusted publisher
- IQ-12 automate npm publish workflow

## Current Blockers
- External npm account/org configuration required (Trusted Publisher linkage).
- Existing GOAP state indicates expired token history; automation depends on fresh publisher setup.

## Mitigation Plan
1. Create external-owner checklist (npm org + GitHub repo mapping + package scope).
2. Validate via dry-run publish workflow in PR context.
3. Keep fallback token path documented but disabled by default once OIDC passes.

## CI Gate Rule
- IQ-11/12 marked `blocked(external-account)` until evidence of successful OIDC claim is attached.
