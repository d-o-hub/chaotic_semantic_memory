import re
import os

def fix_file(path):
    with open(path, 'r') as f:
        content = f.read()

    # 1. Add Hypervector import if not present
    if 'use crate::hyperdim::Hypervector;' not in content and 'use crate::hyperdim::{' not in content:
        # Try to find a good place to insert it
        content = re.sub(r'use crate::error::Result;', 'use crate::error::Result;\nuse crate::hyperdim::Hypervector;', content)

    # 2. Ensure all impl ChaoticSemanticFramework are generic
    content = re.sub(
        r'impl ChaoticSemanticFramework {',
        r'impl<H: Hypervector> ChaoticSemanticFramework<H> {',
        content
    )
    # Handle absolute paths if any
    content = re.sub(
        r'impl crate::framework::ChaoticSemanticFramework {',
        r'impl<H: Hypervector> crate::framework::ChaoticSemanticFramework<H> {',
        content
    )

    with open(path, 'w') as f:
        f.write(content)

# Fix framework extension files
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

for p in files:
    if os.path.exists(p):
        fix_file(p)

# Fix CLI commands - use HVec10240 for now to maintain behavior
with open('src/cli/commands/mod.rs', 'r') as f:
    content = f.read()
content = content.replace('Result<ChaoticSemanticFramework>', 'Result<ChaoticSemanticFramework<HVec10240>>')
with open('src/cli/commands/mod.rs', 'w') as f:
    f.write(content)

with open('src/cli/commands/query.rs', 'r') as f:
    content = f.read()
content = content.replace('framework: &crate::framework::ChaoticSemanticFramework', 'framework: &crate::framework::ChaoticSemanticFramework<crate::hyperdim::HVec10240>')
with open('src/cli/commands/query.rs', 'w') as f:
    f.write(content)
