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

# Correct Hamming distance and similarity in brute_force.rs
fix_file('src/index/brute_force.rs', [
    ('query.hamming_distance(v)', 'query.hamming_distance(v)'), # No change needed if types match
    ('let similarity = 1.0 - (dist as f32 / 5120.0);', 'let similarity = query.cosine_similarity(&self.vectors[idx]);')
])

# Ensure Hypervector is imported in all framework extensions
framework_extensions = [
    'src/framework_bridge.rs', 'src/framework_events.rs', 'src/framework_graph_rag.rs',
    'src/framework_namespaces.rs', 'src/framework_ops.rs', 'src/framework_persistence.rs',
    'src/framework_rerank.rs', 'src/framework_validation.rs', 'src/framework_ttl.rs'
]
for p in framework_extensions:
    if not os.path.exists(p): continue
    with open(p, 'r') as f:
        content = f.read()
    if 'use crate::hyperdim::Hypervector;' not in content and 'Hypervector' not in content:
        content = content.replace('use crate::hyperdim::HVec10240;', 'use crate::hyperdim::{HVec10240, Hypervector};')
    with open(p, 'w') as f:
        f.write(content)

# Fix singularity_search helpers to be generic
fix_file('src/singularity_search.rs', [
    ('query: &HVec10240,', 'query: &H,'),
    ('fn try_cache_lookup<H: Hypervector>(', 'fn try_cache_lookup<H: Hypervector>('),
    ('fn try_ann_lookup<H: Hypervector>(', 'fn try_ann_lookup<H: Hypervector>(')
])
