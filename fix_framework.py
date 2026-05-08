import re
path = 'src/framework.rs'
with open(path, 'r') as f:
    content = f.read()

# Remove the duplicated validation methods I added to framework.rs
start_marker = 'pub(crate) fn validate_concept_id(id: &str)'
end_marker = 'let _ = self.event_sender.send(event); \n    }'
# Using a more robust regex for the block I added
content = re.sub(r'pub\(crate\) fn validate_concept_id.*?let _ = self\.event_sender\.send\(event\);\s+\}', '', content, flags=re.DOTALL)

with open(path, 'w') as f:
    f.write(content)
