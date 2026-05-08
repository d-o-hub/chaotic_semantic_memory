import re
import os

def fix_lsh():
    with open('src/index/lsh.rs', 'r') as f:
        content = f.read()

    # Correct generic usage in LshIndex
    content = content.replace('concepts: HashMap<String, HVec10240>,', 'concepts: HashMap<String, H>,')
    content = content.replace('fn compute_hash(&self, vec: &HVec10240,', 'fn compute_hash(&self, vec: &H,')
    content = content.replace('memory_usage_bytes: self.concepts.len()\n                * (std::mem::size_of::<String>() + std::mem::size_of::<HVec10240>())',
                              'memory_usage_bytes: self.concepts.len()\n                * (std::mem::size_of::<String>() + std::mem::size_of::<H>())')

    # Generic hash computation - use to_bytes() for portability across hypervector formats
    # LSH typically samples bits. We'll sample bytes for generic.
    new_compute_hash = """    fn compute_hash(&self, vec: &H, table_idx: usize) -> u64 {
        let mut hash = 0u64;
        let bits = &self.projections[table_idx];
        let bytes = vec.to_bytes();
        for (i, &bit_pos) in bits.iter().enumerate() {
            let byte_idx = (bit_pos / 8).min(bytes.len() - 1);
            let bit_in_byte = bit_pos % 8;
            if (bytes[byte_idx] & (1u8 << bit_in_byte)) != 0 {
                hash |= 1u64 << i;
            }
        }
        hash
    }"""
    content = re.sub(r'fn compute_hash.*?\}', new_compute_hash, content, flags=re.DOTALL)

    with open('src/index/lsh.rs', 'w') as f:
        f.write(content)

fix_lsh()
