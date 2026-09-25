import urllib.request
import urllib.parse
import json

def search_crossref(query):
    # Using the crossref API
    url = f'https://api.crossref.org/works?query={urllib.parse.quote(query)}&select=title,author,created,URL&rows=20&sort=created&order=desc'
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

search_crossref('hyperchaotic map')
