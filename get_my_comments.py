import urllib.request
import json
import os

repo = 'd-o-hub/chaotic_semantic_memory'
branch = 'feat/graph-rag-hybrid-retrieval'

def safe_urlopen(url):
    req = urllib.request.Request(url)
    return urllib.request.urlopen(req)

# Try to find the PR
url = f'https://api.github.com/repos/{repo}/pulls?head=d-o-hub:{branch}'
print(f"Searching for PR with head {branch}...")
try:
    with safe_urlopen(url) as response:
        data = json.loads(response.read().decode())
        if data:
            pr_number = data[0]['number']
            print(f"Found PR #{pr_number}")

            # Get comments
            url = f'https://api.github.com/repos/{repo}/issues/{pr_number}/comments'
            with safe_urlopen(url) as response:
                comments = json.loads(response.read().decode())
                for comment in comments:
                    print(f"[{comment['user']['login']}]: {comment['body']}")
        else:
            print("No PR found for this branch")
except Exception as e:
    print(f"Error: {e}")
