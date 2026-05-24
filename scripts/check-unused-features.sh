#!/usr/bin/env bash
set -euo pipefail

python - <<'PY'
import glob
import re
import tomllib
from pathlib import Path

cargo = tomllib.loads(Path('Cargo.toml').read_text(encoding='utf-8'))
features = sorted(k for k in cargo.get('features', {}).keys() if k != 'default')

sources = []
for pattern in ('src/**/*.rs', 'tests/**/*.rs', 'benches/**/*.rs', 'examples/**/*.rs'):
    sources.extend(glob.glob(pattern, recursive=True))

text_chunks = []
for path in sources:
    try:
        text_chunks.append(Path(path).read_text(encoding='utf-8'))
    except UnicodeDecodeError:
        continue

haystack = '\n'.join(text_chunks)
unused = []
for feature in features:
    pattern = re.compile(rf'feature\s*=\s*"{re.escape(feature)}"')
    if not pattern.search(haystack):
        unused.append(feature)

if unused:
    print('Unused Cargo features (declared but never checked via #[cfg(feature = ...)]):')
    for feature in unused:
        print(f'  - {feature}')
    raise SystemExit(1)

print(f'ok: all {len(features)} declared non-default features have at least one cfg(feature = ...) usage')
PY
