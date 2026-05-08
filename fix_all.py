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

# Fix brute_force.rs
fix_file('src/index/brute_force.rs', [
    ('vectors: Vec<HVec10240>,', 'vectors: Vec<H>,'),
    ('memory_usage_bytes: self.indices.len()\n                * (std::mem::size_of::<String>() + std::mem::size_of::<HVec10240>() + 16),',
     'memory_usage_bytes: self.indices.len()\n                * (std::mem::size_of::<String>() + std::mem::size_of::<H>() + 16),'),
    ('let similarity = 1.0 - (dist as f32 / 5120.0);', 'let similarity = query.cosine_similarity(&self.vectors[idx]);'),
    ('impl<H: Hypervector> Default for BruteForce<H> { fn default() -> Self { Self { concept_indices: Vec::new(), concept_vectors: Vec::new(), id_to_index: std::collections::HashMap::new(), _phantom: std::marker::PhantomData } } }',
     'impl<H: Hypervector> Default for BruteForce<H> { fn default() -> Self { Self { indices: Vec::new(), vectors: Vec::new(), id_to_index: std::collections::HashMap::new() } } }')
])

# Fix hnsw.rs - for now make it only support HVec10240 or handle generic better
# Actually, hnsw_rs needs a concrete type. We can use HVec10240 as the internal type for HnswIndex<H>
# but it will only work if we can convert H to HVec10240.
# A better way is to make HnswIndex work on H directly.

fix_file('src/index/hnsw.rs', [
    ('Hnsw<\'static, HVec10240, HammingDist>', 'Hnsw<\'static, H, HammingDist<H>>'),
    ('impl Distance<HVec10240> for HammingDist {', 'impl<H: Hypervector> Distance<H> for HammingDist<H> {'),
    ('fn eval(&self, va: &[HVec10240], vb: &[HVec10240])', 'fn eval(&self, va: &[H], vb: &[H])'),
    ('struct HammingDist;', 'struct HammingDist<H: Hypervector>(std::marker::PhantomData<H>);'),
    ('impl<H: Hypervector> HnswIndex<H> {', 'impl<H: Hypervector + \'static> HnswIndex<H> {'),
    (' HammingDist ', ' HammingDist<H> '),
    (' HammingDist,', ' HammingDist<H>,')
])
