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

# Fix graph_rag re-exports
fix_file('src/retrieval/mod.rs', [
    ('graph_rag_retrieve_generic', 'graph_rag_retrieve_generic')
])

# Fix framework_graph_rag.rs imports
fix_file('src/framework_graph_rag.rs', [
    ('use crate::hyperdim::Hypervector;', 'use crate::hyperdim::Hypervector;'),
    ('use crate::retrieval::{GraphRagConfig, GraphRagResult, graph_rag_retrieve};',
     'use crate::retrieval::{GraphRagConfig, GraphRagResult, graph_rag_retrieve_generic};')
])

# Fix index/lsh.rs expected value macro
fix_file('src/index/lsh.rs', [
    ('counts[i] > 0', 'counts[i] > 0') # check for weirdness
])

# Ensure Hypervector trait is used for HVec10240 methods
# We need to import Hypervector trait in files using HVec10240::random() or .to_bytes()
# Or better, re-export them as inherent methods in hvec.rs (I already did this partly)

# Fix singularity_search.rs type annotations
fix_file('src/singularity_search.rs', [
    ('fn try_cache_lookup<H: Hypervector>(', 'fn try_cache_lookup<H: Hypervector + \'static>('),
    ('fn try_ann_lookup<H: Hypervector>(', 'fn try_ann_lookup<H: Hypervector + \'static>(')
])
