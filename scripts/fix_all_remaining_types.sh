#!/bin/bash
# Fix all remaining type inference errors by adding explicit type annotations

cd /home/do/git/d-o-hub/chaotic_semantic_memory

# Fix all files with `let framework = ChaoticSemanticFramework::builder()`
for file in tests/open_issue_api.rs tests/metrics_cache.rs tests/framework_unit.rs tests/batch_ops_coverage.rs tests/framework_ops_coverage.rs tests/builder_advanced.rs tests/cache_operations.rs tests/edge_case_coverage.rs tests/validation_errors.rs tests/batch_operations.rs; do
    if [ -f $file ]; then
        # Add explicit type annotation: let framework: ChaoticSemanticFramework<HVec10240>
        sed -i 's/let framework = ChaoticSemanticFramework::builder()/let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()/g' $file
        echo Fixed: $file
    fi
done

echo Done fixing all files