import urllib.request
import json
import os

repo = os.environ.get('GITHUB_REPOSITORY', 'd-o-hub/chaotic_semantic_memory')
pr_number = os.environ.get('PR_NUMBER', '')

if not pr_number:
    # Try to find the PR
    req = urllib.request.Request(f'https://api.github.com/repos/{repo}/pulls?head=d-o-hub:add-bundle-concepts-strict-tests')
    try:
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            if data:
                pr_number = data[0]['number']
                print(f"Found PR #{pr_number}")
    except Exception as e:
        print(f"Error finding PR: {e}")

if pr_number:
    req = urllib.request.Request(f'https://api.github.com/repos/{repo}/issues/{pr_number}/comments')
    try:
        with urllib.request.urlopen(req) as response:
            comments = json.loads(response.read().decode())
            for comment in comments:
                print(f"[{comment['user']['login']}]: {comment['body']}")
    except Exception as e:
        print(f"Error getting comments: {e}")
else:
    print("Could not determine PR number")
