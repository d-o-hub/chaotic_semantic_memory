#!/bin/bash
# Fix test files to add explicit HVec10240 type parameter to framework builders

set -e

cd /home/do/git/d-o-hub/chaotic_semantic_memory

# Files that need the pattern fix
files=(
    tests/ttl_lifecycle.rs
    tests/framework_lifecycle.rs
    tests/framework_bridge_coverage.rs
    tests/import_export_coverage.rs
    tests/batch_ops_coverage.rs
    tests/eviction_cache.rs
    tests/framework_unit.rs
    tests/builder_config.rs
    tests/path_validation_errors.rs
    tests/import_adversarial.rs
    tests/metrics_cache.rs
    tests/framework_core.rs
    tests/batch_operations.rs
    tests/open_issue_api.rs
    tests/cache_lru_coverage.rs
    tests/validation_errors.rs
    tests/critical_error_paths.rs
    tests/framework_ops_coverage.rs
    tests/edge_case_coverage.rs
    tests/cache_operations.rs
    tests/builder_advanced.rs
    tests/cache_lru.rs
    tests/export_import_coverage.rs
)

for file in ${files[@]}; do
    if [ -f $file ]; then
        # Replace pattern while preserving indentation
        sed -i 's/^\t*let framework = ChaoticSemanticFramework::builder()/\tlet framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()/g' $file
        echo \"Fixed: $file\"
    fi
done

echo \"Done fixing test files\"