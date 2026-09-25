import urllib.request
import urllib.parse
import json
import time

def search_crossref(query, filter_date):
    # Search papers between LAST_RUN_DATE and THIS_RUN_DATE
    url = f'https://api.crossref.org/works?query={urllib.parse.quote(query)}&filter=from-created-date:{filter_date[0]},until-created-date:{filter_date[1]}&select=title,author,created,URL&rows=20&sort=created&order=desc'
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
    'hyperchaotic map locality-sensitive hashing',
    'echo state network associative memory',
    'chaotic projection hyperdimensional',
    'quantization-aware similarity search'
]

for q in queries:
    print(f"\nQuery: {q}")
    search_crossref(q, ('2026-09-18', '2026-09-25'))
    time.sleep(1)
