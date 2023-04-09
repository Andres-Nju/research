import sys
import time
import os

import github3

root_dir = "Commits"
TIME_FOR_SLEEP = 1
pr_key_words = ["fix", "defect", "error", "bug", "issue", "mistake", "incorrect","fault", "flaw"]

def get_repository(gh, owner:str, repo_name:str):
    #print(owner)
    #print(repo_name)
    return gh.repository(owner, repo_name)

def get_short_pull_requests(repo, state:str):
    return repo.pull_requests(state=state)

def contain_key_words(src:str, key_words:list):
    for key in key_words:
        for str in src.split(' '):
            if key in src.lower():
                return True
    return False

if __name__ == '__main__':

    cur_path = os.getcwd() # 当前目录
    # 创建总文件夹
    root_dir = cur_path + "/" + root_dir
    if not os.path.exists(root_dir):
        os.mkdir(root_dir)

    # login
    my_token = "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"
    gh = github3.login(token=my_token)
    
    #get the repo
    repo_file = sys.argv[1]
    with open(repo_file, 'r') as f:
        for repo_full_name in f.readlines():
            owner = repo_full_name.split('/')[0].strip()
            repo_name = repo_full_name.split('/')[1].strip()
            repo = get_repository(gh, owner, repo_name)
            cnt = 0
            repo_dir = root_dir
            # 遍历short pull requests
            for short_pr in get_short_pull_requests(repo, "closed"):
                #print("Pull Request #{0}: {1}".format(short_pr.number, short_pr.title))
                # print("Tag: {0}".format(short_pr.head.ref))
                # 获得对应的pull request
                pr = repo.pull_request(short_pr.number)
                if pr.is_merged() and (contain_key_words(short_pr.head.ref, pr_key_words) or contain_key_words(short_pr.title, pr_key_words)):
                    for commit in pr.commits():
                        # 在这里判断修改行数？
                        print(cnt)
                        cnt = cnt + 1
                        with open(repo_dir + '/' + repo_name + ".txt", 'a') as c:
                            c.write(commit.sha + '\n')
                            time.sleep(TIME_FOR_SLEEP)





    
    
   