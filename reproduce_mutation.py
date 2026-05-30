import sys
import subprocess
import os

def run_tests():
    result = subprocess.run(["cargo", "test", "bm25", "--", "--quiet"], capture_output=True, text=True)
    return result.returncode == 0

filepath = "src/retrieval/bm25.rs"
with open(filepath, "r") as f:
    content = f.read()

# Apply mutation
mutated_content = content.replace("d_idx == idx as u32", "d_idx != idx as u32")

if mutated_content == content:
    print("Could not find mutation point")
    sys.exit(1)

with open(filepath, "w") as f:
    f.write(mutated_content)

try:
    print("Running tests with mutation...")
    if run_tests():
        print("MUTANT SURVIVED! (Tests passed but should have failed)")
    else:
        print("Mutant caught! (Tests failed as expected)")
finally:
    with open(filepath, "w") as f:
        f.write(content)
