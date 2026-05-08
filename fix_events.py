import re
path = 'src/framework_events.rs'
with open(path, 'r') as f:
    content = f.read()

# Fix enum variants - remove <H> from names
content = re.sub(r'Concept<H>(Injected|Updated|Deleted)', r'Concept\1', content)

# Update constructors in the file
content = content.replace('MemoryEvent::Concept<H>Injected', 'MemoryEvent::ConceptInjected')
content = content.replace('MemoryEvent::Concept<H>Updated', 'MemoryEvent::ConceptUpdated')
content = content.replace('MemoryEvent::Concept<H>Deleted', 'MemoryEvent::ConceptDeleted')

with open(path, 'w') as f:
    f.write(content)
