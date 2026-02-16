#!/usr/bin/env python3
import json
import sys
from pathlib import Path

ACTIONS = {
    "analyze-repo": {
        "preconditions": ["repo_access"],
        "effects": ["state_snapshot"],
    },
    "define-acceptance": {
        "preconditions": ["requirements_known"],
        "effects": ["acceptance_defined"],
    },
    "compute-gaps": {
        "preconditions": ["state_snapshot", "acceptance_defined"],
        "effects": ["missing_tasks"],
    },
    "delegate-specialists": {
        "preconditions": ["missing_tasks"],
        "effects": ["specialist_handoffs"],
    },
    "implement-changes": {
        "preconditions": ["specialist_handoffs"],
        "effects": ["artifacts_updated"],
    },
    "add-tests": {
        "preconditions": ["artifacts_updated"],
        "effects": ["tests_updated"],
    },
    "record-adr": {
        "preconditions": ["architecture_decisions_identified"],
        "effects": ["adrs_recorded"],
    },
    "run-verification": {
        "preconditions": ["tests_updated", "adrs_recorded"],
        "effects": ["verification_passed"],
    },
    "run-example": {
        "preconditions": ["verification_passed"],
        "effects": ["example_proven"],
    },
    "finalize-release": {
        "preconditions": ["example_proven"],
        "effects": ["release_ready"],
    },
}

ORDER = [
    "analyze-repo",
    "define-acceptance",
    "compute-gaps",
    "delegate-specialists",
    "implement-changes",
    "add-tests",
    "record-adr",
    "run-verification",
    "run-example",
    "finalize-release",
]

def main() -> int:
    if len(sys.argv) < 3:
        print("usage: build_goap_plan.py <goal> <output_path>")
        return 1

    goal = sys.argv[1]
    output_path = Path(sys.argv[2])
    output_path.parent.mkdir(parents=True, exist_ok=True)

    plan = {
        "goal": goal,
        "actions": [
            {
                "name": action,
                "preconditions": ACTIONS[action]["preconditions"],
                "effects": ACTIONS[action]["effects"],
            }
            for action in ORDER
        ],
    }

    output_path.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    print(str(output_path))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
