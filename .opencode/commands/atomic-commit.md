---
description: Create an atomic git commit with conventional format
---

STRICT RULES - Do not proceed until ALL checks pass:

1. Current working tree status:
!`git status`

2. Staged and unstaged changes:
!`git diff`
!`git diff --staged`

3. Recent commit messages for format reference:
!`git log --oneline -5`

VALIDATION CHECKLIST (you MUST verify each):

- [ ] Changes are atomic (single concern, single responsibility)
- [ ] NO mixed concerns (e.g., don't mix refactor + feature + bugfix)
- [ ] Message follows: <type>(<scope>): <description>
- [ ] Description is imperative, under 72 chars
- [ ] Body explains "why", not "what" (what is in the diff)
- [ ] Breaking changes noted with "BREAKING CHANGE:" footer
- [ ] References issues if applicable with "Closes #N"

VALID TYPES: feat, fix, refactor, test, docs, chore, perf, ci, style, revert
VALID SCOPES: See recent commits for valid scopes in this repo

FORBIDDEN PATTERNS (reject commit if present):
- "update", "changed", "modified" in description (use imperative)
- Multiple type prefixes in one commit
- Files from unrelated features staged together
- Empty diffs or no actual code changes
- Commit messages over 72 chars on subject line

Process:
1. If NON-ATOMIC: reject and explain which files should split into separate commits
2. If ATOMIC but needs work: ask user to refine before committing
3. If VALID: stage files and commit with message
