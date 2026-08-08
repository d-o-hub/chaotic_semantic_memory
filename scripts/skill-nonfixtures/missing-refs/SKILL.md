---
name: missing-refs
description: "Test fixture that references a file which does not exist."
---

# Missing Refs Fixture

This skill points at `references/ghost.md` which was never created. The
validator must flag the dangling reference and exit non-zero.
