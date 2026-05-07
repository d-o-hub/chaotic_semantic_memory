import re

with open('src/singularity.rs', 'r') as f:
    content = f.read()

content = content.replace('Option<&Concept>', 'Option<&Concept<H>>')

# Remove duplicate similarity_cache_key
parts = content.split('pub(crate) fn similarity_cache_key')
if len(parts) > 2:
    content = parts[0] + 'pub(crate) fn similarity_cache_key' + parts[1]

with open('src/singularity.rs', 'w') as f:
    f.write(content)
