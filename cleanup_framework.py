import re
path = 'src/framework.rs'
with open(path, 'r') as f:
    content = f.read()

# Remove the redundant validation block I added earlier
content = re.sub(r'pub\(crate\) fn validate_concept_id\(id: &str\).*?emit_event\(&self, event: MemoryEvent\) \{.*?\}', '', content, flags=re.DOTALL)

# Fix any lingering backslashes
content = content.replace('\&', '&')

with open(path, 'w') as f:
    f.write(content)
