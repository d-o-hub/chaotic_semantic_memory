---
name: skill-creator
description: "Create new skills with proper structure and evals. Use when creating domain-specific skills for the .agents/skills/ directory."
---

# Skill Creator

Template-driven skill creation with evaluation criteria.

## When to Use

- Creating new domain-specific skills
- Standardizing workflow patterns
- Documenting best practices for reuse

## Do NOT Use

- For one-off tasks (use direct implementation)
- When existing skill covers the need

## Process

```
┌─────────────────────────────────────────────────┐
│  1. DEFINE: Identify trigger conditions         │
│  2. DRAFT: Write SKILL.md from template         │
│  3. ADD: Include evaluation criteria            │
│  4. TEST: Validate skill loads correctly        │
└─────────────────────────────────────────────────┘
```

## Directory Structure

```
.agents/skills/<skill-name>/
  SKILL.md          # Required: skill definition
  examples/         # Optional: usage examples
  templates/        # Optional: file templates
  evals.md          # Optional: evaluation criteria
```

## SKILL.md Template

```markdown
---
name: <skill-name>
description: "<one-line description of when to use this skill>"
---

# <Skill Name>

## When to Use
- Bullet list of trigger conditions
- Be specific about use cases

## Do NOT Use
- Anti-patterns and edge cases
- When other skills are more appropriate

## Process
Step-by-step instructions with:
1. Numbered steps
2. Code examples where helpful
3. Clear decision points

## Parameters
| Parameter | Default | Description |
|-----------|---------|-------------|
| key | value | explanation |

## Examples
Concrete usage scenarios

## References
Links to related files (@file.md style)
```

## Step-by-Step Creation

### 1. Define Trigger Conditions

Identify when this skill should activate:
- What user requests match?
- What code patterns trigger it?
- What context is required?

### 2. Write SKILL.md

Create the file following the template:

```bash
mkdir -p .agents/skills/<skill-name>
touch .agents/skills/<skill-name>/SKILL.md
```

Key sections:
- **Frontmatter**: `name` and `description` (required)
- **When to Use**: Specific trigger conditions
- **Process**: Clear, actionable steps
- **Examples**: Concrete usage patterns

### 3. Add Evaluation Criteria

If skill produces artifacts, define success criteria:

```markdown
# Evaluation Criteria

| Criterion | Pass | Fail |
|-----------|------|------|
| Correctness | Output compiles | Syntax errors |
| Completeness | All steps done | Missing steps |
| Quality | Follows patterns | Deviates from style |
```

### 4. Test Skill Loading

```bash
# Verify frontmatter parses
grep -E "^name:|^description:" .agents/skills/<skill-name>/SKILL.md

# Check file structure
ls -la .agents/skills/<skill-name>/
```

## Frontmatter Requirements

| Field | Required | Format |
|-------|----------|--------|
| `name` | Yes | kebab-case, matches directory |
| `description` | Yes | One line, describes trigger |

## Naming Conventions

| Pattern | Example | Use Case |
|---------|---------|----------|
| `<domain>-<action>` | `rust-development` | Domain-specific work |
| `<tool>-<task>` | `github-ci-guardrails` | Tool-specific tasks |
| `swarm-<focus>` | `swarm-testing-quality` | Swarm coordination |
| `self-<action>` | `self-fix-loop` | Automation loops |

## Example: Creating a Skill

```
User: "Create a skill for managing database migrations"

Agent: [Skill Creator activated]
1. DEFINE: Triggers on "migration", "schema change", "db upgrade"
2. DRAFT: Created .agents/skills/db-migrations/SKILL.md
3. ADD: Added evals.md with success criteria
4. TEST: Verified frontmatter and structure

Created skill with:
- When to use: Schema changes, version bumps
- Process: 1) Generate migration 2) Test 3) Apply
- Evals: All migrations reversible, tests pass
```

## Quality Checklist

- [ ] Frontmatter has `name` and `description`
- [ ] Name matches directory name
- [ ] Clear trigger conditions
- [ ] Process has numbered steps
- [ ] Examples are concrete
- [ ] No duplicate of existing skill