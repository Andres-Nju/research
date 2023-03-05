import requests
import json
import sys
import time
import csv
import os
from pydriller import Repository
from pydriller import ModificationType
from urllib.parse import parse_qs, urlparse

root_dir = "Code"
LINES_THRESH = 10
 
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

    cur_path = os.getcwd() # 当前目录
    # 创建总文件夹
    root_dir = cur_path + "/" + root_dir
    if not os.path.exists(root_dir):
        os.mkdir(root_dir)

    #get the repo
    repo_file = sys.argv[1]
    with open(repo_file, 'r') as f:
        for repo_name in f.readlines():
            repo_dir = root_dir + '/' + repo_name.split('/')[1].strip()
            if not os.path.exists(repo_dir):
                os.mkdir(repo_dir)
            repo_name = "https://github.com/" + repo_name.strip()            
            for commit in Repository(repo_name, only_modifications_with_file_types=['.rs']).traverse_commits():
            #for commit in Repository(repo_name).traverse_commits():
                #print(commit.hash[:10])
                if commit.lines > LINES_THRESH:
                    continue
                commit_dir = repo_dir + '/' + commit.hash[:10].strip()
                if os.path.exists(commit_dir):
                    continue
                os.mkdir(commit_dir)
                '''
                Commit:
                deletions (int): number of deleted lines in the commit (as shown from –shortstat).
                insertions (int): number of added lines in the commit (as shown from –shortstat).
                lines (int): total number of added + deleted lines in the commit (as shown from –shortstat).
                files (int): number of files changed in the commit (as shown from –shortstat).

                ModifiedFile:
                added_lines: number of lines added
                deleted_lines: number of lines removed
                '''
                # 根据msg做过滤
                print(commit.msg)

                for modified_file in commit.modified_files:
                    # changed_methods && methods_before
                    # if '.rs' in modified_file.filename and modified_file.content_before is not None and modified_file.content is not None: # 需要改动前和改动后的文件都不为None
                    if '.rs' in modified_file.filename and (modified_file.change_type == ModificationType.RENAME or modified_file.change_type == ModificationType.MODIFY):
                        file_dir = commit_dir + '/' + modified_file.filename.split('.')[0].strip()
                        if os.path.exists(file_dir):
                            continue
                        os.mkdir(file_dir)
                        # 获取改动前后的文件中都出现的method
                        filtered_methods = filter_methods(modified_file.methods_before, modified_file.changed_methods) # name, start_line, end_line
                        for filtered_method in filtered_methods:
                            # 获取改动前的method源代码
                            mehod_before_str = get_content(modified_file.content_before.decode(), filtered_method.start_line_before, filtered_method.end_line_before)
                            #print(mehod_before_str)
                            with open(file_dir + '/' + filtered_method.name + '_before.rs', 'w') as m1:
                                m1.write(mehod_before_str)
                            # 获取改动后的method源代码
                            method_str = get_content(modified_file.content.decode(), filtered_method.start_line_changed, filtered_method.end_line_changed)
                            #print(method_str)
                            with open(file_dir + '/' + filtered_method.name + '_after.rs', 'w') as m2:
                                m2.write(method_str)

                            # 记录改动的方法名
                            with open(file_dir + "/methods.txt", 'a') as method_file:
                                method_file.write(filtered_method.name + '\n')




        
        
        '''sha_token = "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"
        repo_name = sys.argv[1] 
        query_url = 'https://api.github.com/search/repositories?q=' + repo_name
        project_url = 'https://github.com/' + repo_name
        repo_info = fetchUrl(query_url, sha_token)["items"][0]
        time.sleep(1)       
        print(repo_info) '''

    
   