import os
import re

def fix_file(path, replacements):
    if not os.path.exists(path): return
    with open(path, 'r') as f:
        content = f.read()
    for src, dst in replacements:
        content = content.replace(src, dst)
    with open(path, 'w') as f:
        f.write(content)

# Add Hypervector imports where missing
for p in ['src/singularity.rs', 'src/singularity_ext.rs', 'src/singularity_retrieval.rs', 'src/singularity_search.rs']:
    fix_file(p, [('use crate::hyperdim::HVec10240;', 'use crate::hyperdim::{HVec10240, Hypervector};')])

# Fix search functions in singularity_search.rs
with open('src/singularity_search.rs', 'r') as f:
    content = f.read()
content = content.replace('fn try_cache_lookup(', 'fn try_cache_lookup<H: Hypervector>(')
content = content.replace('fn try_ann_lookup(', 'fn try_ann_lookup<H: Hypervector>(')
with open('src/singularity_search.rs', 'w') as f:
    f.write(content)
