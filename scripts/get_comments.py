import urllib.request
import json
import os

repo = os.environ.get('GITHUB_REPOSITORY', 'd-o-hub/chaotic_semantic_memory')
pr_number = os.environ.get('PR_NUMBER', '')

def safe_urlopen(url):
    if not url.startswith('https://api.github.com/'):
        raise ValueError(f"URL {url} is not allowed")
    req = urllib.request.Request(url)
    return urllib.request.urlopen(req)

if not pr_number:
    # Try to find the PR
    url = f'https://api.github.com/repos/{repo}/pulls?head=d-o-hub:feat/reranking-pipeline-v2'
    try:
        with safe_urlopen(url) as response:
            data = json.loads(response.read().decode())
            if data:
                pr_number = data[0]['number']
                print(f"Found PR #{pr_number}")
    except Exception as e:
        print(f"Error finding PR: {e}")

if pr_number:
    url = f'https://api.github.com/repos/{repo}/issues/{pr_number}/comments'
    try:
        with safe_urlopen(url) as response:
            comments = json.loads(response.read().decode())
            for comment in comments:
                print(f"[{comment['user']['login']}]: {comment['body']}")
    except Exception as e:
        print(f"Error getting comments: {e}")
else:
    print("Could not determine PR number")
