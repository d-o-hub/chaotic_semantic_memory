import sys
import re

def normalize(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # Replace non-deterministic timestamp
    content = re.sub(r'^Generated: .*$', 'Generated: [TIMESTAMP REMOVED FOR DETERMINISM]', content, flags=re.MULTILINE)

    # Handle Dependencies and Features lists
    lines = content.splitlines(keepends=True)
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith('**Dependencies:**') or line.startswith('**Features:**'):
            new_lines.append(line)
            i += 1
            items = []
            while i < len(lines) and lines[i].startswith('- '):
                items.append(lines[i])
                i += 1
            new_lines.extend(sorted(items))
        else:
            new_lines.append(line)
            i += 1
    content = "".join(new_lines)

    # Split by ## sections
    # We use a lookahead to keep the delimiter with the following part
    sections = re.split(r'^(## .*)$', content, flags=re.MULTILINE)

    header = sections[0]
    sec_map = {} # header -> body

    # Standard sections we want to keep at the top
    standard_order = ["## Table of Contents", "## Core Documentation", "## README.md"]
    other_sections = []

    for k in range(1, len(sections), 2):
        s_header = sections[k].strip()
        s_body = sections[k+1]

        # Normalize the body by sorting ### subsections
        subsections = re.split(r'^(### .*)$', s_body, flags=re.MULTILINE)
        normalized_body = subsections[0]
        blocks = []
        for j in range(1, len(subsections), 2):
            blocks.append((subsections[j], subsections[j+1]))
        blocks.sort()
        for bh, bb in blocks:
            normalized_body += bh + bb

        # Clean up excessive separators/newlines
        normalized_body = re.sub(r'\n---\n', '\n', normalized_body)

        if s_header in standard_order:
            sec_map[s_header] = normalized_body
        else:
            other_sections.append((s_header, normalized_body))

    # Sort other sections (like ## src/...)
    other_sections.sort()

    final_content = [header]
    for s in standard_order:
        if s in sec_map:
            final_content.append(s + "\n" + sec_map[s])

    for s_header, s_body in other_sections:
        final_content.append(s_header + "\n" + s_body)

    # Final cleanup: ensure single trailing newline and no double-dashes left strangely
    result = "".join(final_content)
    result = re.sub(r'\n{3,}', '\n\n', result)

    with open(filename, 'w') as f:
        f.write(result)

if __name__ == '__main__':
    normalize(sys.argv[1])
