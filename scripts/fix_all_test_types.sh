#!/bin/bash
# Fix all remaining test files to add explicit HVec10240 type parameter

set -e

cd /home/do/git/d-o-hub/chaotic_semantic_memory

echo 'Fixing test files...'

# Process each test file
for file in tests/*.rs; do
    if [ -f \"$file\" ]; then
        # Fix framework builder pattern - handle various indentation levels
        sed -i 's/let framework = ChaoticSemanticFramework::builder()/let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()/g' \"$file\"
        echo \"Fixed: $file\"
    fi
done

# Fix Singularity type parameter in tests
echo ''
echo 'Fixing Singularity type parameters...'
for file in tests/*.rs; do
    if [ -f \"$file\" ]; then
        # Fix Singularity::with_config and Singularity::new
        sed -i 's/let mut singularity = Singularity::with_config/let mut singularity: Singularity<HVec10240> = Singularity::with_config/g' \"$file\"
        sed -i 's/let mut singularity = Singularity::new/let mut singularity: Singularity<HVec10240> = Singularity::new/g' \"$file\"
    fi
done

echo ''
echo 'Done fixing test files'