---
name: failing-check
description: "Test fixture whose check.sh exits non-zero; its status must be propagated."
---

# Failing Check Fixture

This skill ships an executable `check.sh` that always exits 1. The runner
must execute it exactly once, propagate the failure, and exit non-zero
(no `|| true`, no masked failures).
