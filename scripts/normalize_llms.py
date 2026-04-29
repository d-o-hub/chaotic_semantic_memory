import sys

def normalize(filename):
    with open(filename, 'r') as f:
        content = f.read()

    lines = content.splitlines(keepends=True)
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith('**Dependencies:**'):
            new_lines.append(line)
            i += 1
            items = []
            while i < len(lines) and lines[i].startswith('- '):
                items.append(lines[i])
                i += 1
            new_lines.extend(sorted(items))
            continue
        elif line.startswith('**Features:**'):
            new_lines.append(line)
            i += 1
            items = []
            while i < len(lines) and lines[i].startswith('- '):
                items.append(lines[i])
                i += 1
            new_lines.extend(sorted(items))
            continue
        elif line.startswith('Generated:'):
            # Remove the non-deterministic generation timestamp
            new_lines.append('Generated: [TIMESTAMP REMOVED FOR DETERMINISM]\n')
            i += 1
            continue
        else:
            new_lines.append(line)
            i += 1

    with open(filename, 'w') as f:
        f.writelines(new_lines)

if __name__ == '__main__':
    normalize(sys.argv[1])
