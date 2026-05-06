import urllib.request
import json
import os
import subprocess

repo = os.environ.get('GITHUB_REPOSITORY', 'd-o-hub/chaotic_semantic_memory')
branch = subprocess.check_output(['git', 'rev-parse', '--abbrev-ref', 'HEAD']).decode().strip()

def safe_urlopen(url):
    if not url.startswith('https://api.github.com/'):
        raise ValueError(f"URL {url} is not allowed")
    req = urllib.request.Request(url)
    token = os.environ.get('GITHUB_TOKEN')
    if token:
        req.add_header('Authorization', f'token {token}')
    return urllib.request.urlopen(req)

url = f'https://api.github.com/repos/{repo}/pulls?head=d-o-hub:{branch}'
pr_number = ''
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

    url = f'https://api.github.com/repos/{repo}/pulls/{pr_number}/comments'
    try:
        with safe_urlopen(url) as response:
            comments = json.loads(response.read().decode())
            for comment in comments:
                print(f"REVIEW [{comment['user']['login']}]: {comment['body']}")
    except Exception as e:
        print(f"Error getting review comments: {e}")
else:
    print("Could not determine PR number")
