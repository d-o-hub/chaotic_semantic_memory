import re
path = 'src/singularity.rs'
with open(path, 'r') as f:
    content = f.read()

# Ensure #[serde(bound = ...)] is correctly placed above the struct
content = re.sub(r'#\[serde\(bound = "H: Hypervector"\)\]\s+#\[derive', '#[derive', content)
content = re.sub(r'pub struct Concept<H: Hypervector = HVec10240>', '#[serde(bound = "H: Hypervector")]\npub struct Concept<H: Hypervector = HVec10240>', content)

with open(path, 'w') as f:
    f.write(content)
