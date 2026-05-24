import urllib.request
import json
import os

repo = os.environ.get('GITHUB_REPOSITORY', 'd-o-hub/chaotic_semantic_memory')
pr_number = os.environ.get('PR_NUMBER', '')

def safe_urlopen(url):
    req = urllib.request.Request(url)
    # Adding a header might be needed for some API calls if they require it,
    # but usually public repos are fine.
    # Actually, jules environment might have GITHUB_TOKEN.
    token = os.environ.get('GITHUB_TOKEN')
    if token:
        req.add_header('Authorization', f'token {token}')
    return urllib.request.urlopen(req)

if pr_number:
    url = f'https://api.github.com/repos/{repo}/pulls/{pr_number}/comments'
    try:
        with safe_urlopen(url) as response:
            comments = json.loads(response.read().decode())
            for comment in comments:
                print(f"[{comment['user']['login']} at {comment['path']}:{comment.get('line')}]: {comment['body']}")
    except Exception as e:
        print(f"Error getting review comments: {e}")
else:
    print("Could not determine PR number")
