import re
import os

def fix_framework_extensions():
    files = [
        'src/framework_bridge.rs',
        'src/framework_events.rs',
        'src/framework_graph_rag.rs',
        'src/framework_namespaces.rs',
        'src/framework_ops.rs',
        'src/framework_persistence.rs',
        'src/framework_rerank.rs',
        'src/framework_validation.rs',
        'src/framework_ttl.rs'
    ]

    for path in files:
        if not os.path.exists(path):
            continue
        with open(path, 'r') as f:
            content = f.read()

        # 1. Ensure Generic Impl
        if 'impl<H: Hypervector> ChaoticSemanticFramework<H>' not in content:
            content = content.replace('impl ChaoticSemanticFramework {', 'impl<H: Hypervector> ChaoticSemanticFramework<H> {')
            content = content.replace('impl crate::framework::ChaoticSemanticFramework {', 'impl<H: Hypervector> crate::framework::ChaoticSemanticFramework<H> {')

        # 2. Add necessary imports
        if 'use crate::hyperdim::Hypervector;' not in content and 'use crate::hyperdim::{' not in content:
            if 'use crate::error::Result;' in content:
                content = content.replace('use crate::error::Result;', 'use crate::error::Result;\nuse crate::hyperdim::Hypervector;')
            elif 'use crate::error::{' in content:
                content = re.sub(r'use crate::error::\{([^}]*)\};', r'use crate::error::{\1};\nuse crate::hyperdim::Hypervector;', content)

        # 3. Replace HVec10240 with H in method signatures within the impl block
        # This is risky but often correct for framework methods
        # Avoid replacing HVec10240 where it's part of a Result<HVec10240> for reservoir
        # We'll do targeted replacements

        # Concept -> Concept<H>
        content = re.sub(r'(?<!<)Concept(?!<)', 'Concept<H>', content)
        # Fix double Concept<H><H> or similar if occurred
        content = content.replace('Concept<H><H>', 'Concept<H>')

        # In framework_ops, bundle_concepts_strict returns HVec10240?
        # No, it should probably return H if it uses singularity.
        # But if it uses reservoir it returns HVec10240.

        # Let's fix the most common ones
        content = content.replace('vector: HVec10240', 'vector: H')
        content = content.replace('query: &HVec10240', 'query: &H')
        content = content.replace('-> Result<HVec10240>', '-> Result<H>')

        # Special case: src/framework_ops.rs has some specific ones
        if 'framework_ops.rs' in path:
            content = content.replace('pub async fn bundle_concepts_strict(&self, ids: &[String]) -> Result<H>', 'pub async fn bundle_concepts_strict(&self, ids: &[String]) -> Result<H>')

        with open(path, 'w') as f:
            f.write(content)

def fix_singularity_ext():
    # singularity_ext.rs and singularity_retrieval.rs might need alignment too
    files = ['src/singularity_ext.rs', 'src/singularity_retrieval.rs', 'src/singularity_search.rs']
    for path in files:
        if not os.path.exists(path):
            continue
        with open(path, 'r') as f:
            content = f.read()

        if 'impl<H: Hypervector + \'static> Singularity<H>' not in content:
            content = content.replace('impl Singularity {', 'impl<H: Hypervector + \'static> Singularity<H> {')

        content = re.sub(r'(?<!<)Concept(?!<)', 'Concept<H>', content)
        content = content.replace('Concept<H><H>', 'Concept<H>')
        content = content.replace('vector: HVec10240', 'vector: H')
        content = content.replace('query: &HVec10240', 'query: &H')
        content = content.replace('-> Result<HVec10240>', '-> Result<H>')

        with open(path, 'w') as f:
            f.write(content)

fix_framework_extensions()
fix_singularity_ext()
