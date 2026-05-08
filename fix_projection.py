import re
path = 'src/embedding/projection.rs'
with open(path, 'r') as f:
    content = f.read()

assertion = '        assert!(config.target_dim == 10240, "target_dim must be 10240 for HVec10240, got {}", config.target_dim);\n'
content = content.replace('let mut entries = Vec::new();', assertion + '        let mut entries = Vec::new();')

with open(path, 'w') as f:
    f.write(content)
