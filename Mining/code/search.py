import requests
import json
import sys
import time
import csv
from pydriller import Repository
from urllib.parse import parse_qs, urlparse

def get_commits_count(repo:str) -> int:
    """
    Returns the number of commits to a GitHub repository.
    """
    headers = {
        'User-Agent':'Andres-Nju',
        'Authorization':"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",
        'Content-Type':'application/json',
        'method':'GET',
        'Accept':'application/json'
    }
    url = f"https://api.github.com/repos/" + repo + "/commits?per_page=1"
    r = requests.get(url, headers=headers)
    links = r.links
    #print(links)
    rel_last_link_url = urlparse(links["last"]["url"])
    rel_last_link_url_args = parse_qs(rel_last_link_url.query)
    #print(rel_last_link_url_args)
    rel_last_link_url_page_arg = rel_last_link_url_args["page"][0]
    commits_count = int(rel_last_link_url_page_arg)
    return commits_count

def get_closed_issues_count(repo:str):
    """
    Returns the number of commits to a GitHub repository.
    """
    headers = {
        'User-Agent':'Andres-Nju',
        'Authorization':"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",
        'Content-Type':'application/json',
        'method':'GET',
        'Accept':'application/json'
    }
    #https://api.github.com/search/issues?q=repo:nodejs/node+type:issue+state:closed
    url = f"https://api.github.com/search/issues?q=repo:" + repo + "+type:issue+state:closed"
    r = requests.get(url, headers=headers)
    r.raise_for_status()
    result = json.loads(r.text)
    #print(result)
    return result["total_count"]


# 获取指定接口的数据
def fetchUrl(url, sha_token):
    '''
    功能：访问 url 的网页，获取网页内容并返回
    参数：目标网页的 url
    返回：目标网页的 html 内容
    '''
 
    headers = {
        'User-Agent':'Andres-Nju',
        'Authorization':sha_token,
        'Content-Type':'application/json',
        'method':'GET',
        'Accept':'application/json'
    }
 
    r = requests.get(url, headers=headers)
    r.raise_for_status()
    r.encoding = r.apparent_encoding
 
    result = json.loads(r.text)  # json字符串转换成字典
 
    return result
 
# 获取source code指定行的内容
def get_content(source_code:str, start_line:int, end_line:int):
    res = ""
    for s in source_code.split('\n')[start_line - 1:end_line]:
        res = res + s + "\n"
    return res

class MyMethod:
    def __init__(self, name, start_line_before, end_line_before, start_line_changed, end_line_changed) -> None:
        self.name = name
        self.start_line_before = start_line_before
        self.end_line_before = end_line_before
        self.start_line_changed = start_line_changed
        self.end_line_changed = end_line_changed


# 获取改动前后的methods中的同名methods，返回新的method_before和changed_method
def filter_methods(methods_before, changed_methods):
    res = []
    for method_before in methods_before:
        for changed_method in changed_methods:
            if method_before.name == changed_method.name:
                res.append(MyMethod(method_before.name, method_before.start_line, method_before.end_line, changed_method.start_line, changed_method.end_line))
    return res

        



 
if __name__ == '__main__':
    '''
    主函数：程序入口
    '''
    '''sha_token = "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"
    repo_name = sys.argv[1]
    
    with open("corpus.csv",'w') as corpus:
        csv_writer = csv.writer(corpus)
        #csv_writer.writerow(["full_name", "description", "open_issues", "closed_issues", "commits", "last_update"])
        with open(repo_name, 'r') as repos_file:
            for repo in repos_file.readlines():
                repo = repo.strip()
                print(repo)
                query_url = 'https://api.github.com/search/repositories?q=' + repo
                project_url = 'https://github.com/' + repo
                repo_info = fetchUrl(query_url, sha_token)["items"][0]
                time.sleep(1)
                closed_issue_count = get_closed_issues_count(repo)
                time.sleep(1)
                commits_count = get_commits_count(repo)
                time.sleep(1)
                csv_writer.writerow([repo_info["full_name"], repo_info["description"], repo_info["open_issues"], closed_issue_count, commits_count, repo_info["pushed_at"], project_url])
                # print(repo_info["full_name"])
                # print(repo_info["description"])
                # print(repo_info["open_issues"])
                # print(get_closed_issues_count(repo))
                # print(get_commits_count(repo))
                # print(repo_info["pushed_at"])'''


    
    #get the repo
    repo_name = sys.argv[1]
    repo_name = "https://github.com/" + repo_name
    for commit in Repository(repo_name, only_modifications_with_file_types=['.rs']).traverse_commits():
    #for commit in Repository(repo_name).traverse_commits():
        print(commit.lines)

        '''
        deletions (int): number of deleted lines in the commit (as shown from –shortstat).
        insertions (int): number of added lines in the commit (as shown from –shortstat).
        lines (int): total number of added + deleted lines in the commit (as shown from –shortstat).
        files (int): number of files changed in the commit (as shown from –shortstat).
        '''

        for modified_file in commit.modified_files:
            # changed_methods && methods_before
            if modified_file.content_before is not None and modified_file.content is not None: # 需要改动前和改动后的文件都不为None
                # 获取改动前后的文件中都出现的method
                filtered_methods = filter_methods(modified_file.methods_before, modified_file.changed_methods) # name, start_line, end_line
                for filtered_method in filtered_methods:
                    # 获取改动前的method源代码
                    mehod_before_str = get_content(modified_file.content_before.decode(), filtered_method.start_line_before, filtered_method.end_line_before)
                    print(mehod_before_str)
                    # 获取改动后的method源代码
                    method_str = get_content(modified_file.content.decode(), filtered_method.start_line_changed, filtered_method.end_line_changed)
                    print(method_str)


    
    
    '''sha_token = "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"
    repo_name = sys.argv[1] 
    query_url = 'https://api.github.com/search/repositories?q=' + repo_name
    project_url = 'https://github.com/' + repo_name
    repo_info = fetchUrl(query_url, sha_token)["items"][0]
    time.sleep(1)       
    print(repo_info) '''

    
   