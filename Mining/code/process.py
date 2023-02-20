import os

root_dir = 'ASTs'


if __name__ == '__main__':
    cur_path = os.getcwd() # 当前目录
    # 创建总文件夹
    root_dir = cur_path + "/" + root_dir
    if not os.path.exists(root_dir):
        os.mkdir(root_dir)
    '''with open("repos.txt", 'r') as repos:
        for repo in repos.readlines():
            repo_name = repo.split('/')[1].strip()'''

    os.system("rustc code_to_ast/src/main.rs")
    
    code_dir = cur_path + '/Codes' 

    with os.scandir(code_dir) as Codes:
        for repo in Codes: # repo level
            repo_dir = root_dir + '/' + repo.name
            os.mkdir(repo_dir)
            with os.scandir(repo.path) as Commmits:
                for commit in Commmits:
                    commit_dir = repo_dir + '/' + commit.name
                    os.mkdir(commit_dir)
                    with os.scandir(commit.path) as files:
                        for file in files:
                            file_dir = commit_dir + '/' + file.name
                            os.mkdir(file_dir)
                            with os.scandir(file.path) as methods:
                                for method in methods:
                                    if ".txt" == method.name[-4:]:
                                        continue
                                    method_file = file_dir + '/' + method.name
                                    #os.system("rustc code_to_ast/src/main.rs")
                                    os.system("./main " + method.path + " " + method_file)
                            
