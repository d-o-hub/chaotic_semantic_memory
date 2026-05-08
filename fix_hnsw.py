import re
path = 'src/index/hnsw.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix HammingDist struct
content = content.replace('struct HammingDist<H: Hypervector>(std::marker::PhantomData<H>);', 'struct HammingDist<H: Hypervector>(std::marker::PhantomData<H>);')

# Find first 'fn deserialize' and replace it until the end of the impl block
# Actually, the file structure seems messy now. Let's just fix the end of the file.

new_impl_end = """
    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        use std::fs;
        if data.is_empty() { return Ok(()); }
        let wrapper: HnswPersistenceWrapper = bincode::deserialize(data).map_err(|e| MemoryError::database(format!("Bincode deserialize fail: {}", e)))?;
        let temp_dir = tempfile::tempdir().map_err(MemoryError::Io)?;
        fs::write(temp_dir.path().join("index.hnsw.data"), &wrapper.data).map_err(MemoryError::Io)?;
        fs::write(temp_dir.path().join("index.hnsw.graph"), &wrapper.graph).map_err(MemoryError::Io)?;
        let loader = HnswIo::new(temp_dir.path(), "index");
        let hnsw = loader.load_hnsw_with_dist::<H, HammingDist<H>>(HammingDist(std::marker::PhantomData))
            .map_err(|e| MemoryError::database(format!("HNSW load failed: {}", e)))?;

        let static_hnsw: Hnsw<'static, H, HammingDist<H>> = unsafe { std::mem::transmute(hnsw) };
        self.hnsw = static_hnsw;
        self._temp_dir = Some(temp_dir);
        self.id_to_idx = wrapper.id_to_idx;
        self.idx_to_id = wrapper.idx_to_id;
        self.config.m = wrapper.m;
        self.config.ef_construction = wrapper.ef_construction;
        self.config.ef_search = wrapper.ef_search;
        self.deleted_count = wrapper.deleted_count;
        Ok(())
    }
}
"""

content = re.sub(r'fn deserialize.*?$', new_impl_end, content, flags=re.DOTALL)

with open(path, 'w') as f:
    f.write(content)
