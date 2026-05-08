import re
path = 'src/framework_graph_rag.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix imports
content = content.replace('use crate::hyperdim::{HVec10240, Hypervector};', 'use crate::hyperdim::Hypervector;')
if 'use crate::hyperdim::Hypervector;' not in content:
    content = content.replace('use crate::hyperdim::HVec10240;', 'use crate::hyperdim::Hypervector;')

# Ensure retrieval generic is used
content = content.replace('graph_rag_retrieve(', 'crate::retrieval::graph_rag::graph_rag_retrieve_generic::<H>(')

with open(path, 'w') as f:
    f.write(content)
