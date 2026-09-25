import urllib.request
import urllib.parse
import json

def get_fallback(query):
    # Search papers published BEFORE the last run date (2026-09-18)
    url = f'https://api.crossref.org/works?query={urllib.parse.quote(query)}&filter=until-created-date:2026-09-18&select=title,author,created,URL&rows=5&sort=created&order=desc'
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        response = urllib.request.urlopen(req)
        data = json.loads(response.read())

        for item in data['message']['items']:
            title = item.get('title', [''])[0]
            authors = [a.get('family', '') + ' ' + a.get('given', '') for a in item.get('author', [])]
            date = item.get('created', {}).get('date-time', '')
            link = item.get('URL', '')

            print(f"Title: {title}")
            print(f"Published: {date}")
            print(f"Authors: {', '.join(authors)}")
            print(f"Link: {link}")
            print("-" * 40)
    except Exception as e:
        print(f"Error: {e}")

queries = [
    'chaotic map locality-sensitive hashing',
]

for q in queries:
    print(f"\nQuery: {q}")
    get_fallback(q)
