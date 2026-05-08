import re
path = 'src/singularity_retrieval.rs'
with open(path, 'r') as f:
    content = f.read()

new_func = """    /// Generate candidates by coarse bucketing.
    pub(crate) fn generate_bucket_candidates(&self, ns: &str, _query: &H) -> Vec<usize> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Vec::new();
        };
        (0..ns_state.concept_vectors.len()).collect()
    }"""

content = re.sub(r'/// Generate candidates by coarse bucketing\..*?pub\(crate\) fn exact_similarity_scan', new_func + "\n\n    /// Perform exact similarity scan over all vectors.\n    pub(crate) fn exact_similarity_scan", content, flags=re.DOTALL)

with open(path, 'w') as f:
    f.write(content)
