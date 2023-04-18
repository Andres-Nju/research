import sys
import time
import os
import subprocess
import json

root_dir = "Commits"
TIME_FOR_SLEEP = 1


def get_repository(gh, owner:str, repo_name:str):
    #print(owner)
    #print(repo_name)
    return gh.repository(owner, repo_name)

def get_short_pull_requests(repo, state:str):
    return repo.pull_requests(state=state)

def contain_key_words(src:str, key_words:list):
    for key in key_words:
        for str in src.split(' '):
            if key == str.lower():
                return True
    return False

keywords = ["fix", "defect", "error", "bug", "issue", "mistake", "incorrect", "flaw"]




if __name__ == '__main__':
    # 创建总文件夹
    root_dir = os.getcwd() + "/" + root_dir
    if not os.path.exists(root_dir):
        os.mkdir(root_dir)

    #get the repo
    repo_file = sys.argv[1]
    with open(repo_file, 'r') as f:
        for repo_full_name in f.readlines():
            owner = repo_full_name.split('/')[0].strip()
            repo_name = repo_full_name.split('/')[1].strip()
            cnt = 0
            print(f'repo: {repo_name}')
            repo_path = root_dir + '/' + repo_name
            # Retrieve pull request using gh pr list
            prs = subprocess.check_output(
                f'cd {repo_path} && gh pr list --state merged --limit 10000000', shell=True).decode().split('\n')[:-1]

            # Loop through each merged commit and check if it belongs to a pull request
            for pr in prs:
                pr_title = pr.split('\t')[1]
                pr_num = pr.split('\t')[0]

                # filter certain pr
                if not contain_key_words(pr_title, keywords):
                    '''pr_labels = json.loads(subprocess.check_output(f'cd {repo_path} && gh pr view {pr_num} --json labels', shell=True).decode())
                    flag = False
                    for label in pr_labels:
                        if contain_key_words(label, keywords):
                            flag = True
                            break
                    if not flag:
                        continue'''
                    continue
                while True:
                    try:
                        pr_merge_commit = json.loads(subprocess.check_output(f'cd {repo_path} && gh pr view {pr_num} --json mergeCommit', shell=True).decode())
                        #print(pr_commits)
                        if pr_merge_commit is not None:
                            if 'mergeCommit' in pr_merge_commit.keys():
                                if pr_merge_commit['mergeCommit'] is not None:
                                    if 'oid' in pr_merge_commit['mergeCommit'].keys():
                                        with open(root_dir + '/' + repo_name + ".txt", 'a') as c:
                                            c.write(pr_merge_commit['mergeCommit']['oid'] + '\n')
                                            cnt = cnt + 1
                        break
                    except:
                        continue
            print(cnt)
        

