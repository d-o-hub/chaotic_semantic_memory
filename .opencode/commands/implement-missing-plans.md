---
description: Generic workflow to check plans/ tasks against codebase, implement missing items, then atomic commit/push with CI verification loop
---

STRICT RULES - Loop until ALL GitHub Actions pass (non-skipped):

## PREREQUISITES

Verify environment:
!`gh auth status 2>&1 | head -5`
!`git rev-parse --abbrev-ref HEAD`

FAIL if on main/master branch - create feature branch first.

## STEP 1: Discover Pending Tasks from Plans

1. Generate combined agents first (ensures @impl, @fix, @plan, @test, @ci, @perf, @swarm exist):
   !`scripts/generate-agents.sh`

2. Use plans-manager.sh for quick status:
   !`scripts/plans-manager.sh status`
   !`scripts/plans-manager.sh count`

3. Read plans structure:
   !`ls -la plans/`
   !`ls plans/adr/*.md 2>/dev/null | wc -l`

4. Extract pending actions from plans/ACTIONS.md:
   !`grep -i "status: pending" plans/ACTIONS.md | wc -l`
   !`grep -B2 "status: pending" plans/ACTIONS.md | grep -E "^\s+- name:" | head -20`

5. Extract incomplete states from plans/GOAP_STATE.md:
   !`grep -E ":\s*(false|no|0|null|incomplete)$" plans/GOAP_STATE.md | grep -v "^#" | head -20`

6. Validate plans files:
!`scripts/plans-manager.sh validate`

7. Check for pre-existing GitHub issues not in plans:
!`gh issue list --state open --limit 50 --json number,title,body | jq -r '.[] | "Issue #\(.number): \(.title)"' | head -20`
!`gh issue list --state open --limit 50 | wc -l`

8. Cross-reference issues with ACTIONS.md to find untracked work:
```bash
# Check if open issues are reflected in pending actions
gh issue list --state open --json number,title | jq -r '.[].title' | while read title; do
  if ! grep -qi "$title" plans/ACTIONS.md 2>/dev/null; then
    echo "UNTRACKED: Issue '$title' not found in ACTIONSn  fi
done
```

## STEP 1.5: Ensure GOAP and ADR Coverage

For each discovered task (from plans + issues):

1. Verify GOAP_STATE.md entry exists:
!`grep -c "<task_name>" plans/GOAP_STATE.md`

2. If GOAP state missing, add it:
```markdown
<task_name>:
  status: incomplete
  priority: high
  depends_on: []
  adr_required: true|false
```

3. Check if ADR exists (if required by task):
!`ls plans/adr/*.md 2>/dev/null | xargs grep -l "<task_name>" 2>/dev/null | wc -l`

4. If ADR required but missing:
- Create ADR using `adr-creation` skill
- Link to ACTIONS.md entry
- Update ADR_REGISTRY.md

5. Track all tasks in IMPLEMENTATION_QUEUE - NOTHING SKIPPED:
```bash
# Verify no gaps between discovered work and queue
TOTAL_DISCOVERED=$(echo "$PLAN_TASKS $ISSUE_TASKS" | wc -w)
QUEUED=$(echo "$IMPLEMENTATION_QUEUE" | wc -w)
if [ "$TOTAL_DISCOVERED" -ne "$QUEUED" ]; then
  echo "ERROR: Tasks discovered ($TOTAL_DISCOVERED) != Tasks queued ($QUEUED)"
  exit 1
fi
```

## STEP 2: Validate Current Codebase State

Run validation gates (stop on first failure):
!`cargo fmt -- --check`
!`cargo clippy --all-targets --all-features -- -D warnings`

```bash
# Run tests and verify at least 1 test executed
TEST_OUTPUT=$(cargo test --all-features 2>&1)
TEST_COUNT=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= tests?)' | tail -1)
if [ -z "$TEST_COUNT" ] || [ "$TEST_COUNT" -eq 0 ]; then
  echo "FAIL: No tests executed (count: ${TEST_COUNT:-0})"
  exit 1
fi
echo "PASS: $TEST_COUNT tests executed"
```

LOC_FAIL=0
for file in $(find src -name '*.rs'); do
  LOC=$(wc -l < "$file")
  if [ "$LOC" -gt 500 ]; then
    echo "FAIL: $file exceeds 500 LOC ($LOC lines)"
    LOC_FAIL=1
  fi
done
if [ "$LOC_FAIL" -eq 1 ]; then exit 1; fi

## STEP 3: Analyze Gap Between Plans and Code

For each pending task from STEP 1:
1. Read action definition: !`grep -A10 "- name: <task_name>" plans/ACTIONS.md`
2. Check ADR if referenced: !`cat plans/adr/<adr-number>.md 2>/dev/null | head -30`
3. Identify target file(s): !`grep "^    file:" plans/ACTIONS.md | grep <task_name>`
4. Verify implementation exists: !`grep -l "<function_name>" src/**/*.rs 2>/dev/null`

Build IMPLEMENTATION_QUEUE of all missing items.

## STEP 3.5: Group Tasks for Parallel Execution

Group IMPLEMENTATION_QUEUE into parallel execution groups:

| Group | Task Types | Agents | Max Parallel |
|-------|------------|--------|--------------|
| A | Code implementation | @impl | 2 |
| B | Bug fixes | @fix | 2 |
| C | Performance | @perf | 1 |
| D | Testing | @test | 2 |
| E | Planning/ADR | @plan | 1 |
| F | Research | websearch | 1 |

Create handoff contract:
```
Group A -> Group B: <shared assumptions>
Group B -> Group C: <performance findings>
...
```

## STEP 4: Spawn Specialist Agents (Based on Gap Analysis)

Categorize IMPLEMENTATION_QUEUE using combined agents:
- **needs_code**: @impl agent (rust-development + testing-validation)
- **needs_fix**: @fix agent (rust-development + testing-validation + debugging-reservoir)
- **needs_test**: @test agent (testing-validation + swarm-testing-quality)
- **needs_plan**: @plan agent (goap-planning + adr-creation)
- **needs_ci**: @ci agent (github-ci-guardrails + git-workflow)
- **needs_perf**: @perf agent (benchmarking-perf + debugging-reservoir + swarm-performance)
- **needs_swarm**: @swarm agent (all swarm skills)
- **needs_research**: websearch + general agent

Execute categories in parallel where independent:
!`echo "Parallel agents: $(echo $IMPLEMENTATION_QUEUE | wc -w) tasks queued"`

## STEP 5: Implement Each Missing Task

For each task in IMPLEMENTATION_QUEUE:
1. Read ADR/specification fully
2. Use appropriate combined agent (@impl, @fix, @perf, @test, @plan)
3. Run targeted validation:
```bash
cargo check || exit 1
TEST_OUTPUT=$(cargo test --lib 2>&1)
TEST_COUNT=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= tests?)' | tail -1)
if [ -z "$TEST_COUNT" ] || [ "$TEST_COUNT" -eq 0 ]; then
  echo "FAIL: No tests executed in --lib (count: ${TEST_COUNT:-0})"
  exit 1
fi
```
4. If validation fails: fix and retry (max 3 attempts per task)
5. Do NOT skip or defer - always implement

TRACK_PROGRESS: Maintain list of completed vs failed tasks.

## STEP 5.5: Handoff Coordination Between Groups

After each group completes their tasks:

1. Generate handoff document:
```bash
# Create plans/handoffs/<GROUP>_to_<GROUP>_<task>.md
```

Handoff template:
```markdown
# Handoff: Group <X> -> Group <Y>

## Completed Tasks
- <task1>: <summary>
- <task2>: <summary>

## Assumptions Passed
- <assumption1>
- <assumption2>

## Findings for Next Group
- <finding1>
- <finding2>

## Validation Results
- <validation1>
```

2. Read prior handoffs for dependencies:
   !`ls plans/handoffs/`

3. Document handoff in SWARM_COORDINATION.md:
   - Update group status
   - Add handoff artifact reference

## STEP 6: Update Plans to Reflect Completion

After each task completes:
- Update ACTIONS.md: status: pending → status: complete
- Update GOAP_STATE.md: set appropriate true/false
- Update ADR if architectural change made

### Update plans/ with New Agent System

1. Add new agent mappings to SWARM_COORDINATION.md:
   - Document @impl, @fix, @perf, @test, @plan, @ci, @swarm groups
   - Map to existing skills

2. Archive old ADRs if needed:
   !`scripts/plans-manager.sh archive adr`

3. Update progress files:
   !`scripts/plans-manager.sh truncate progress/LEARNINGS.md 200`
   !`scripts/plans-manager.sh truncate progress/PROGRESS.md 200`

Use plans-manager.sh for maintenance:
!`scripts/plans-manager.sh sync`
!`scripts/plans-manager.sh validate`

For self-learning (when files get too large):
!`scripts/plans-manager.sh truncate progress/LEARNINGS.md 200`
!`scripts/plans-manager.sh truncate progress/PROGRESS.md 200`
!`scripts/plans-manager.sh archive adr`  # Keep last 10 ADRs

## STEP 7: Final Validation Before Commit

Run full validation gates:
```bash
cargo fmt -- --check || exit 1
cargo clippy --all-targets --all-features -- -D warnings || exit 1

# Run tests and verify at least 1 test executed
TEST_OUTPUT=$(cargo test --all-features 2>&1)
TEST_COUNT=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= tests?)' | tail -1)
if [ -z "$TEST_COUNT" ] || [ "$TEST_COUNT" -eq 0 ]; then
  echo "FAIL: No tests executed in final validation (count: ${TEST_COUNT:-0})"
  exit 1
fi
echo "PASS: Final validation - $TEST_COUNT tests executed"
```

If any fail: return to STEP 5 to fix.

## STEP 8: Atomic Commit and Push (NO AMEND AFTER PUSH)

**ATOMIC COMMIT RULES:**
- ONE initial commit with ALL changes
- Once pushed, NEVER amend - create new fixup commits instead
- Each CI failure loop creates a NEW commit, not an amend
- History must remain linear and immutable after push

1. Verify not on protected branch:
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" = "main" ] || [ "$BRANCH" = "master" ]; then
  echo "ERROR: Cannot commit directly to $BRANCH"
  exit 1
fi

2. Stage ALL changes atomically:
```bash
git add -A
STAGED=$(git diff --cached --stat | wc -l)
echo "Staging $STAGED files atomically"
!`git status --short`
```

3. Create atomic commit with conventional format:
```bash
COMMIT_MSG="fix(plans): implement pending tasks from GOAP state

- Implemented <task1> per ADR-XXXX
- Implemented <task2> per ADR-XXXX
- Updated plans/GOAP_STATE.md
- Updated plans/ACTIONS.md

Co-authored-by: opencode <opencode@local>"

git commit -m "$COMMIT_MSG"
echo "ATOMIC COMMIT CREATED: $(git rev-parse --short HEAD)"
```

4. Push to remote (this is the atomic boundary - no rewrites after this point):
!`git push -u origin "$BRANCH"`
echo "ATOMIC PUSH COMPLETE - History is now immutable for this branch"

## STEP 9: Verify GitHub Actions

1. Get run ID:
   !`gh run list --limit 1 --branch "$BRANCH" | head -5`

2. Wait for CI to start (poll every 10s, max 5 minutes):
   RUN_ID=$(gh run list --limit 1 --branch "$BRANCH" --json id -q '.[0].id')
   echo "Waiting for CI run: $RUN_ID"

   for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
     STATUS=$(gh api repos/$OWNER/$REPO/actions/runs/$RUN_ID -q '.conclusion' 2>/dev/null)
     if [ -n "$STATUS" ]; then break; fi
     echo "Waiting for CI... ($i/30)"
     sleep 10
   done

3. Check result:
   !`gh run view "$RUN_ID" --json conclusion,status -q '.conclusion, .status'`

4. If failed (non-skipped):
- Get logs: !`gh run view "$RUN_ID" --log-failed 2>&1 | head -100`
- Analyze failure, fix in code
- Create NEW fixup commit (NO AMEND): !`git add -A && git commit -m "fix(plans): ci fix iteration $ITERATION" && git push`
- Repeat verification

**NOTE:** Never use `--amend` after push. Each fix creates immutable history.

## STEP 10: Loop Until CI Passes (Max 5 iterations)

MAX_ITERATIONS=5
ITERATION=1

while [ $ITERATION -le $MAX_ITERATIONS ]; do
  echo "=== Iteration $ITERATION/$MAX_ITERATIONS ==="
  
# Run full validation
cargo fmt -- --check || exit 1
cargo clippy --all-targets --all-features -- -D warnings || exit 1

# Run tests and verify at least 1 test executed
TEST_OUTPUT=$(cargo test --all-features 2>&1)
TEST_COUNT=$(echo "$TEST_OUTPUT" | grep -oP '\d+(?= tests?)' | tail -1)
if [ -z "$TEST_COUNT" ] || [ "$TEST_COUNT" -eq 0 ]; then
  echo "FAIL: No tests executed in CI loop iteration $ITERATION (count: ${TEST_COUNT:-0})"
  exit 1
fi
echo "PASS: Iteration $ITERATION - $TEST_COUNT tests executed"

# Commit and push (NO AMEND - create new commit each iteration)
git add -A
git commit -m "fix(plans): iteration $ITERATION fixes - $TEST_COUNT tests passed"
git push
  
  # Verify CI
  RUN_ID=$(gh run list --limit 1 --branch "$BRANCH" --json id -q '.[0].id')
  STATUS=$(gh api repos/$OWNER/$REPO/actions/runs/$RUN_ID -q '.conclusion' 2>/dev/null)
  
  if [ "$STATUS" = "success" ]; then
    echo "CI PASSED after $ITERATION iteration(s)"
    exit 0
  fi
  
  ITERATION=$((ITERATION + 1))
done

echo "ERROR: Max iterations ($MAX_ITERATIONS) reached. CI still failing."
echo "Manual intervention required."
exit 1

## FORBIDDEN PATTERNS

- Hardcoding specific project tasks (must be generic)
- Skipping implementation of any pending task
- Removing features to make tests pass
- Bypassing validation gates
- Force-pushing to main/master
- Amending after push (only amend before push)
- Infinite loops without max iteration cap
