# Sensor Sharpness: gates that report clean because they looked at nothing

Fourteen months of this repo's own incidents collapse into one failure mode: a check whose
pattern, inventory, or dependency silently degraded to *nothing*, and therefore passed. The
rule is simple — **a green from a sensor you cannot prove was measuring is not a verdict** —
but every instance below looked fine in review.

## 1. A pattern that cannot match passes every input

glibc `grep -E` treats a `)` with no matching `(` as a *literal*. So

```bash
grep -E 'git (clone|pull|fetch)'      # fine
grep -E 'git (clone|pull|fetch)' ')'  # a stray paren turns the whole thing into a literal
```

The second form matches nothing, and a gate built on "no match ⇒ no finding" reports clean
forever. Defense: probe every pattern against a known positive and a known negative before
running the real audit, and exit 2 if either probe disagrees.

```bash
probe() {  # probe <pattern> <positive> <negative> — a regex is only trusted once it has
    printf '%s\n' "$2" | grep -qE -- "$1" || return 1   # answered both a match and a miss
    printf '%s\n' "$3" | grep -qE -- "$1" && return 1
    return 0
}
probe "$HOST_RE" 'git clone https://example.invalid' 'echo "git clone is not run here"' \
    || { echo "error: HOST_RE did not behave as written"; exit 2; }
```

Same class, different tool: `awk '…END{exit !found}' | grep -q .` can **never** succeed,
because awk's exit status is consumed by the pipeline and `grep -q .` reads nothing. Use
awk's own status: `awk '…' <<<"$text"` and test `$?`.

## 2. Three exit codes, and never collapse them

| code | meaning | caller's obligation |
|---|---|---|
| `0` | the audit ran and found nothing | safe to treat as a verdict |
| `1` | the audit ran and found a problem | fix or explicitly accept |
| `2` | the audit **could not run** | never a pass; block, and print why |

`2` covers: missing dependency (PyYAML, tomllib, shellcheck), unreadable or absent sibling
script, empty inventory to scan, and a parser disagreeing with a reference parser. The trap
that bit #964's tool: `raise SystemExit("message")` exits **1**, not 2 — so a refusal written
that way reports "drift" instead of "I could not check". Use a single helper:

```python
def refuse(message):
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(2)
```

Bash equivalent of the same lie: `if cmd; then …; fi` leaves `$?` = 0 when the condition
fails. Capture it: `status=0; cmd || status=$?`.

## 3. An empty inventory is a finding, not a pass

#949's nix audit scans the bodies of `nix develop` steps and the scripts they exec. Its
first version treated "zero wrapped entrypoints found" as clean; the tree has three, and if
that list ever emptied, the gate would have printed "no unscrubbed host helper" while
reading nothing. Two changes:

- the pass line **prints what it counted** (`3 wrapped entrypoint script(s) scanned`), so a
  zero is visible in the log and a fixture can assert on it;
- a wrapped step that execs a script the tree cannot read is itself a finding — "cannot
  verify" is not "verified clean".

## 4. Discovery by walk + declared set, not by list

#964's real lesson: `find crates -name Cargo.toml` is a *claim about the repository*, and
the repository grew a member (`benchmarks/`) outside it, which then shipped three releases
behind (`0.3.6`) while every gate said "synchronized". Enumerate the filesystem — a walk
reaches a member before anyone `git add`s it, and needs no git at all — and then ask the
manifest itself which files *must* have been read: cross-check `[workspace] members`,
expanded the way cargo expands it, against the discovered set, and refuse (exit 2) on a
declared member that is missing.

```
a declared workspace member has no manifest among those discovered: target/x/Cargo.toml
— discovery was blind, so this is not a verdict
```

That second half is what makes a prune list honest. `target/` must be pruned (`cargo
package` unpacks a copy of every published crate there, each with its own stale pins) and
`.qoder/` must be pruned (dispatched worktrees are full checkouts of this repository, and
their drift is not editable), so a prune *can* in principle eat a real member — the
declared-set cross-check is what converts "we decided not to look" from a silent green into
a refusal. A silent fallback is the sensor narrowing its own scope while keeping its old
verdict; announcing the fallback on stderr is the weaker version of the same bug, and this
file used to recommend it.

## 5. Neuter the subject, not the fixture

Write the gate, write the fixture, then prove the fixture can fail by breaking the gate one
direction at a time. Recipe (keep a `/tmp` copy and `cmp` after restoring — untracked files
have no `git checkout` to fall back to):

```python
ORIG = open("/tmp/tool.orig.py").read()
for name, old, new in mutations:
    assert ORIG.count(old) == 1          # an anchor that matches twice mutated nothing
    write(SUBJECT, ORIG.replace(old, new, 1))
    run(fixture)                          # every mutation must turn >=1 case red
    shutil.copy("/tmp/tool.orig.py", SUBJECT)
```

Measured results, both sessions:

- #949's gate: 8/8 mutations caught; neutering it turned 8 fixture cases red.
- #964's tool: 9/9 caught (`dev-dependencies` unseen, drift never reported, silent-pin
  cross-check removed, `refuse` exiting 1, index discovery removed, `--sync` not writing,
  member filter removed, multi-line tables ignored, `git ls-files` genuinely broken).
- #964's verifier (`verify-version-sync.sh`): the first sweep let **2 of 4** mutations walk
  away green — "exit 2 downgraded to a pass" and "crates-only member glob restored" — so
  two fixture tests were written for exactly those directions, and the second sweep was 6/6.
- #964's walk discovery (2026-11-01): 4 of 5 caught — `PRUNED_DIRS` emptied, the declared-member
  coverage cross-check neutered, the manifest realpath key dropped, `followlinks=False`. The
  survivor is the directory `seen_dirs` guard, and the reason is the rule: **two redundant
  guards make each other untestable.** Dropping either one still prints the right count,
  because the other collapses the duplicate path. A guard whose failure mode is *wasted work*
  rather than a *wrong verdict* has no exit code that can see it — so isolate it with a case
  only it can pass, or comment the test as insurance. Do not leave a green assertion implying
  sharpness the guard does not have.

A mutation that the fixture survives is not a gap in the mutation list; it is a case that
was never written. The escaping mutation is the finding.

## 6. A mock tree must ship the tool's siblings

`scripts/test-version-sync.sh` copies `verify-version-sync.sh` into a temp directory and
runs it there. The moment the verifier gained a dependency on
`scripts/workspace-version-edges.py`, Test 1 failed for a reason that looked like a
version-sync regression. Two rules: fixtures that copy a script copy its dependencies too,
and a copied-alone verifier should fail **loudly** (`ERROR: … is missing — the pin audit
cannot run`, exit 2), which is also a testable direction of its own.

## 7. Keep fixture helper signatures identical across fixtures

`expect_has "<description>" "<needle>" "<case>"` is the shape used by
`scripts/test-check-nix-git-env.sh`. When a new fixture defined `has "<desc>" "<case>"
"<needle>"` and then called it in the *other* order, 18 cases quietly grepped a
non-existent file and reported a failure that was really a plumbing bug — noise that reads
like findings hides the real ones. Copy a sibling fixture's helpers, or fix the sibling.
