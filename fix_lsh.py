import os
import re

with open('src/index/lsh.rs', 'r') as f:
    content = f.read()

# Fix messed up compute_hash
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
    }
}"""
content = re.sub(r'fn compute_hash.*?impl<H: Hypervector> AnnIndex<H>', new_compute_hash + "\n\nimpl<H: Hypervector> AnnIndex<H>", content, flags=re.DOTALL)

# Fix similarity calculation - use trait method if possible or standard formula
content = content.replace('1.0 - (dist as f32 / 5120.0)', 'query.cosine_similarity(vec)')
# In search_filtered as well
content = content.replace('let dist = query.hamming_distance(&c.vector);\n                    let similarity = 1.0 - (dist as f32 / 5120.0);',
                          'let similarity = query.cosine_similarity(&c.vector);')

with open('src/index/lsh.rs', 'w') as f:
    f.write(content)
