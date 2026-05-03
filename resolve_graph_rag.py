import os

path = 'src/retrieval/graph_rag.rs'
with open(path, 'r') as f:
    lines = f.readlines()

output = []
skip = False
for line in lines:
    if '<<<<<<< HEAD' in line:
        pass
    elif '=======' in line:
        skip = True
    elif '>>>>>>> main' in line:
        skip = False
    elif not skip:
        output.append(line)

with open(path, 'w') as f:
    f.writelines(output)
