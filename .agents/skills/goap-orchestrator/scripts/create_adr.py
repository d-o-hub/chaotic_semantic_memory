#!/usr/bin/env python3
import argparse
from pathlib import Path
from datetime import datetime, UTC

TEMPLATE = """# {title}\n\n- ADR ID: {adr_id}\n- Status: Proposed\n- Context: \n- Decision: \n- Consequences: \n- Alternatives Considered: \n- Validation: \n- Follow-up Actions: \n"""

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--slug", required=True)
    parser.add_argument("--title", required=True)
    parser.add_argument("--dir", default="plans/adrs")
    args = parser.parse_args()

    now = datetime.now(UTC).strftime("%Y%m%d")
    adr_id = f"ADR-{now}-{args.slug}"
    out_dir = Path(args.dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    out_file = out_dir / f"{adr_id}.md"
    out_file.write_text(TEMPLATE.format(title=args.title, adr_id=adr_id), encoding="utf-8")
    print(str(out_file))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
