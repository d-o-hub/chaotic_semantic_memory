import os
path = 'src/reservoir.rs'
with open(path, 'r') as f:
    lines = f.readlines()

new_lines = []
skip = False
for line in lines:
    if 'pub fn to_hypervector' in line:
        new_lines.append('    pub fn to_hypervector(&self) -> Result<HVec10240> {\n')
        new_lines.append('        if self.size < HVec10240::DIMENSION {\n')
        new_lines.append('            return Err(MemoryError::InvalidDimension { expected: HVec10240::DIMENSION, actual: self.size });\n')
        new_lines.append('        }\n')
        new_lines.append('        let chunk_size = self.size / HVec10240::DIMENSION;\n')
        new_lines.append('        let mut data = [0.0f32; 10240];\n')
        new_lines.append('        for (i, val) in data.iter_mut().enumerate() {\n')
        new_lines.append('            let start = i * chunk_size;\n')
        new_lines.append('            let end = start + chunk_size;\n')
        new_lines.append('            let sum: f32 = self.state[start..end].iter().sum();\n')
        new_lines.append('            *val = sum / chunk_size as f32;\n')
        new_lines.append('        }\n')
        new_lines.append('        Ok(HVec10240 { data })\n')
        new_lines.append('    }\n')
        skip = True
        continue
    if skip:
        if line.strip() == '}':
            skip = False
        continue
    new_lines.append(line)

with open(path, 'w') as f:
    f.writelines(new_lines)
