import re
import os

with open('plans/ADR_REGISTRY.md', 'r') as f:
    content = f.read()

# Fix ADR-0024 links (second one)
lines = content.splitlines()
count_0024 = 0
new_lines = []
for line in lines:
    if '[0024]' in line:
        count_0024 += 1
        if count_0024 == 2:
            line = line.replace('0024-concept-expiration-ttl.md', '0024-performance-optimizations-phase2.md')
    if '[0030]' in line:
        line = line.replace('0030-cli-crate-architecture.md', '0030-test-and-benchmark-gap-remediation.md')
    new_lines.append(line)

# Add ADR-0049 if missing from table
found_49 = False
for line in new_lines:
    if '[0049]' in line: found_49 = True
if not found_49:
    # Insert before 0050
    final_lines = []
    for line in new_lines:
        if '[0050]' in line:
            final_lines.append('| [0049](plans/adr/0049-release-checklist-and-version-sync-protocol.md) | Release Checklist & Version Sync | Accepted | Implemented |')
        final_lines.append(line)
    new_lines = final_lines

with open('plans/ADR_REGISTRY.md', 'w') as f:
    f.write('\n'.join(new_lines) + '\n')
