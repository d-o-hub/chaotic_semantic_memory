---
name: triz-analysis
description: "Perform systematic contradiction analysis before implementation. Use when facing design tradeoffs, architectural decisions, or problems where improving one parameter worsens another."
---

# TRIZ Analysis

## When to Use
- Before implementing solutions to complex problems
- When you encounter "either/or" tradeoffs
- When improving one aspect degrades another
- During architectural design decisions

## Process

### 1. Define the Ideal Final Result (IFR)
Ask: "What would the solution look like if it solved itself?"
- Define the ideal outcome without constraints
- Identify what resources already exist
- Consider how the problem could eliminate itself

### 2. Identify Technical Contradictions
For each tradeoff, express as: "If I improve A, then B gets worse."

Common software contradictions:
| Improving | Worsens |
|-----------|---------|
| Performance | Readability |
| Flexibility | Simplicity |
| Security | Usability |
| Features | Maintainability |
| Speed | Memory usage |
| Abstraction | Debuggability |
| Type safety | Verbosity |
| Testability | Coupling |

### 3. Apply Contradiction Matrix
Use `references/principles.md` to map contradictions to inventive principles.

### 4. Generate Solutions
For each suggested principle, brainstorm specific solutions.

### 5. Document Findings
Create a TRIZ analysis in `plans/triz/` with:
- Problem statement
- IFR definition
- Contradictions identified
- Principles applied
- Solution candidates
- Chosen approach

## Example

**Problem**: Batch similarity search is slow, but per-query optimization adds complexity.

**IFR**: Search optimizes itself based on query patterns without explicit branches.

**Contradiction**: Improving speed worsens code simplicity.

**Principles Applied**:
- #1 Segmentation: Split search into configurable strategies
- #2 Taking out: Extract strategy selection to separate module
- #10 Preliminary action: Pre-compute selectivity statistics

**Solution**: Strategy pattern with automatic selection based on pre-computed metrics.