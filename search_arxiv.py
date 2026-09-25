import urllib.request
import urllib.parse
import xml.etree.ElementTree as ET

def search_arxiv(query):
    url = f'http://export.arxiv.org/api/query?search_query={urllib.parse.quote(query)}&sortBy=submittedDate&sortOrder=descending&max_results=20'
    try:
        response = urllib.request.urlopen(url)
        xml_data = response.read()
        root = ET.fromstring(xml_data)

        for entry in root.findall('{http://www.w3.org/2005/Atom}entry'):
            title = entry.find('{http://www.w3.org/2005/Atom}title').text.strip()
            published = entry.find('{http://www.w3.org/2005/Atom}published').text
            authors = [a.find('{http://www.w3.org/2005/Atom}name').text for a in entry.findall('{http://www.w3.org/2005/Atom}author')]
            summary = entry.find('{http://www.w3.org/2005/Atom}summary').text.strip()
            link = entry.find('{http://www.w3.org/2005/Atom}id').text

            print(f"Title: {title}")
            print(f"Published: {published}")
            print(f"Authors: {', '.join(authors)}")
            print(f"Link: {link}")
            print("-" * 40)
    except Exception as e:
        print(f"Error: {e}")

queries = [
    'all:"chaotic map" AND all:"semantic hashing"',
    'all:"chaotic map" AND all:"hyperdimensional"',
    'all:"Echo State Networks" AND all:"retrieval"',
    'all:"Locality-Sensitive Hashing" AND all:"chaotic"',
    'all:"quantization-aware similarity search"',
    'all:"chaotic map" AND all:"locality-sensitive hashing"'
]

for q in queries:
    print(f"\nQuery: {q}")
    search_arxiv(q)
