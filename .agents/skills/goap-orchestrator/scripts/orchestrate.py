#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

SPECIALISTS = [
    "architecture-agent",
    "implementation-agent",
    "test-agent",
    "performance-agent",
    "persistence-agent",
    "wasm-agent",
    "release-agent",
]

TOKEN_RULES = {
    "architecture-agent": ["architecture", "adr", "diagram", "draw.io"],
    "test-agent": ["test", "assert", "coverage", "integration"],
    "performance-agent": ["bench", "latency", "performance", "throughput"],
    "persistence-agent": ["turso", "persist", "restore", "database", "checkpoint"],
    "wasm-agent": ["wasm", "bindgen", "webassembly"],
    "release-agent": ["release", "commit", "pr", "changelog"],
}


def assign_specialist(task: str) -> str:
    key = task.lower()

    for specialist in [
        "architecture-agent",
        "test-agent",
        "performance-agent",
        "persistence-agent",
        "wasm-agent",
        "release-agent",
    ]:
        if any(token in key for token in TOKEN_RULES[specialist]):
            return specialist

    return "implementation-agent"


def build_board(goal: str, actions: list[dict], tasks: list[str]) -> dict:
    board = {
        "goal": goal,
        "goap_actions": actions,
        "specialists": {name: [] for name in SPECIALISTS},
        "unassigned": [],
    }
    for task in tasks:
        specialist = assign_specialist(task)
        if specialist in board["specialists"]:
            board["specialists"][specialist].append(task)
        else:
            board["unassigned"].append(task)
    return board


def render_markdown(board: dict) -> str:
    lines = [
        "# GOAP Orchestration Board",
        "",
        f"- Goal: `{board['goal']}`",
        f"- GOAP actions: `{len(board['goap_actions'])}`",
        "",
        "## Specialist Assignments",
    ]
    for specialist, tasks in board["specialists"].items():
        lines.append(f"### {specialist}")
        if tasks:
            lines.extend([f"- {task}" for task in tasks])
        else:
            lines.append("- (none)")
        lines.append("")
    if board["unassigned"]:
        lines.append("## Unassigned")
        lines.extend([f"- {task}" for task in board["unassigned"]])
        lines.append("")
    lines.append("## Completion Gate")
    lines.append("- [ ] All specialist tasks completed")
    lines.append("- [ ] ADRs updated")
    lines.append("- [ ] Verification commands passed")
    lines.append("- [ ] Runnable example executed")
    lines.append("")
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--goal", required=True)
    parser.add_argument("--plan", required=True, help="GOAP plan json path")
    parser.add_argument(
        "--tasks", required=True, help="JSON file containing array of missing task strings"
    )
    parser.add_argument("--out-json", required=True)
    parser.add_argument("--out-md", required=True)
    return parser


def main_cli(argv: list[str]) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    plan_path = Path(args.plan)
    tasks_path = Path(args.tasks)
    out_json = Path(args.out_json)
    out_md = Path(args.out_md)

    plan_data = json.loads(plan_path.read_text(encoding="utf-8"))
    tasks_data = json.loads(tasks_path.read_text(encoding="utf-8"))

    if not isinstance(tasks_data, list):
        raise SystemExit("tasks file must be a JSON array")

    board = build_board(
        args.goal, plan_data.get("actions", []), [str(item) for item in tasks_data]
    )

    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(board, indent=2) + "\n", encoding="utf-8")
    out_md.write_text(render_markdown(board), encoding="utf-8")

    print(str(out_json))
    print(str(out_md))
    return 0


def main(argv: list[str] | None = None) -> int:
    if argv is None:
        import sys

        argv = sys.argv[1:]
    return main_cli(argv)


if __name__ == "__main__":
    raise SystemExit(main())
