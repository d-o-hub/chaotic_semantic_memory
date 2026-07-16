# Harness Engineering

> **Agent = Model + Harness.** Fail-closed sensors catch violations; guides prevent them.
> Domain: HDC hypervectors · echo-state reservoir · Turso/libSQL · CLI `csm` · WASM
> Spec basis: [Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) · ADR-0090

## Purpose

This harness is a **closed control loop** for agents and humans:

| Axis | Role | Trust |
|------|------|-------|
| **Feedforward (guides)** | What to do *before* coding — constraints, GOAP plan, skills | Inferential (direction) |
| **Feedback (sensors)** | What fires *after* coding — fmt, clippy, deny, tests, arch | Computational (always trust) |

Sensors are **fail-closed**: a red sensor blocks commit/PR until fixed. Agents must not suppress sensors with blanket `#[allow(...)]` or skip gates.

```bash
# Primary agent entrypoint (structured HARNESS VIOLATION output)
./scripts/harness-check.sh <fmt|clippy|deny|test|arch|all>
```

---

## Sensor Map

| Sensor | Command | Checks | Fix hint | Guide / skill |
|--------|---------|--------|----------|---------------|
| **fmt** | `./scripts/harness-check.sh fmt` | rustfmt | `cargo fmt --all` | — |
| **clippy** | `./scripts/harness-check.sh clippy` | `-D warnings` workspace | Fix warnings; see `.clippy.toml`; justify any `#[allow]` | `rust-development` |
| **deny** | `./scripts/harness-check.sh deny` | licenses / bans / advisories / sources | Inspect `deny.toml` for violation type | `github-ci-guardrails` |
| **test** | `./scripts/harness-check.sh test` | workspace tests, all features | Fix the failing test | `testing-validation` |
| **arch** | `./scripts/harness-check.sh arch` | layering + fitness (`tests/arch_fitness.rs`) | Move code to correct layer; read test error | ADR-0090, `arch_fitness` |
| **all** | `./scripts/harness-check.sh all` | fmt → clippy → deny → test → arch | Fix each red sensor in order | this file |

### Broader tool map (not all wrapped by harness-check)

| Tool | Path | Role |
|------|------|------|
| Full local gates | `./scripts/validate.sh` | fmt, clippy, compile-warn-free, tests, **LOC ≤500**, WASM check, size gate, llms.txt |
| Quality runner | `./scripts/quality-gates.sh` | Multi-language gate (`--fix` optional) |
| Mutation score | `./scripts/mutation_test.sh` | cargo-mutants; threshold via `MUTATION_THRESHOLD` (default 85) |
| ADR parity | `./scripts/check-adr-parity.sh` | `plans/ADR_REGISTRY.md` ↔ on-disk ADRs |
| LOC pre-check | `find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + \| sort -rn \| head -20` | Every `.rs` in `src/` + `crates/` ≤ **500** |
| Pre-commit | `./scripts/pre-commit.sh` | Fast local gate before commit |
| CLI surface lock | `cargo test --test cli_parity --features cli` | When touching `src/cli/**` or `src/bin/csm.rs` |

---

## Feedforward Loop (plan → ship)

```text
GOAP plan          implement           validate              PR
─────────    →     ─────────    →     ──────────    →     ────────
plans/GOAP_*       branch +            harness-check         gh pr create
plans/ACTIONS.md   minimal edit        validate.sh           CI green → merge
AGENTS.md / skills                     LOC + ADR parity      (never push main)
```

1. **Plan** — Load `plans/GOAP_STATE.md` + `plans/ACTIONS.md`; scope from uncommitted diff.
2. **Implement** — Feature branch only; match style; read before edit (`AGENTS.md`).
3. **Validate** — `./scripts/harness-check.sh all` then `./scripts/validate.sh`.
4. **PR** — Branch → commit → `gh pr create` → wait for CI → merge. **Never push `main`.**

Skills: `goap-planning`, `rust-development`, `testing-validation`, `git-workflow`, `github-ci-guardrails`.

---

## Feedback Loop (violation → fix)

```text
sensor red  →  structured error  →  minimal fix  →  re-run sensor  →  green
```

1. Sensor exits non-zero with **`❌ HARNESS VIOLATION [name]`**.
2. Read **`AGENT FIX HINT:`** line and the failure body.
3. Apply the **smallest** fix (no drive-by refactors).
4. Re-run **that** sensor only, then `all` before claiming done.
5. If the same sensor fails **>2 times** in a session → update a feedforward guide (AGENTS.md, skill, hard-constraints) so it cannot recur (`learn` skill / compound engineering).

---

## Agent Self-Correction Protocol

`scripts/harness-check.sh` prefixes failures for easy parse:

```text
❌ HARNESS VIOLATION [cargo clippy]

  AGENT FIX HINT: Fix all warnings. Check .clippy.toml for allowed exceptions. ...
  See HARNESS.md for the full sensor ↔ guide map.
```

| Step | Action |
|------|--------|
| 1 | Detect `HARNESS VIOLATION` (or validate.sh / CI red) |
| 2 | Classify: `fmt` \| `clippy` \| `deny` \| `test` \| `arch` \| LOC \| ADR |
| 3 | Follow the fix hint; open the cited config if needed |
| 4 | Re-run: `./scripts/harness-check.sh <sensor>` |
| 5 | Do not open/merge PR until sensors are green |
| 6 | Encode systemic fixes into guides (not one-off silences) |

Success line: `✅ HARNESS OK [name]`.

---

## Domain Constraints (CSM)

| Constraint | Bound | Enforced by |
|------------|-------|-------------|
| Reservoir spectral radius | **[0.9, 1.1]** | Code + review; `debugging-reservoir` |
| Source LOC | **≤ 500** per file (`src/`, `crates/`) | `validate.sh`, `arch_fitness`, pre-session check |
| SKILL.md LOC | **≤ 250** | Convention / skill review |
| Persistence client | **libsql** (not `turso-client`) | Review + hard-constraints |
| Lint policy | `unwrap`/`expect`/`panic` warn (error in CI) | workspace lints, `.clippy.toml` (tests exempt) |
| Git | **Never push `main`** | branch → PR → CI → merge |
| Supply chain | `cargo deny check` before release | `deny.toml`, harness **deny** sensor |

Full list: [`agents-docs/hard-constraints.md`](agents-docs/hard-constraints.md).

---

## Feedforward Guides (read first)

| Guide | Path |
|-------|------|
| Agent contract | `AGENTS.md` |
| Claude/session hooks | `CLAUDE.md` |
| This harness map | `HARNESS.md` |
| Hard constraints | `agents-docs/hard-constraints.md` |
| Clippy policy | `.clippy.toml` |
| Dependency policy | `deny.toml` |
| GOAP world state | `plans/GOAP_STATE.md`, `plans/ACTIONS.md` |
| ADRs | `plans/adr/`, `plans/ADR_REGISTRY.md` |
| Skills | `.agents/skills/*/` (`testing-validation`, `rust-development`, `goap-planning`, …) |
| Quick commands | `agents-docs/quick-reference.md` |

---

## Steering Loop

When a sensor fires repeatedly:

1. Categorize: style · lint · architecture · behaviour · supply-chain · domain physics.
2. Strengthen the matching **guide** (not only the code).
3. Prefer a new skill section or hard-constraint over silent `#[allow]`.
4. Capture patterns in `progress/LEARNINGS.md` when novel.

---

## Minimal Agent Checklist

```bash
# Before claiming done
./scripts/harness-check.sh all
./scripts/validate.sh
./scripts/check-adr-parity.sh
# CLI changes only:
cargo test --test cli_parity --features cli
```

Branch → PR → CI green → merge. Sensors win over intent.
