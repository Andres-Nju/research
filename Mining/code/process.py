import os
import sys
vector_file = "vector_test.csv"

if __name__ == '__main__':
    code_repo = sys.argv[1]
    cur_path = os.getcwd() # 当前目录
    code_dir = cur_path + '/' + code_repo
    with os.scandir(code_dir) as Codes:
        for repo in Codes: # repo level
            with os.scandir(repo.path) as Commmits:
                for commit in Commmits:
                    with os.scandir(commit.path) as files:
                        for file in files:
                            with os.scandir(file.path) as methods:
                                for method in methods:
                                    if ".txt" == method.name[-4:]:
                                        continue
                                    if "before.rs" == method.name[-9:]:
                                        os.system("./difftastic/target/debug/difft --display side-by-side-show-both --context 0 " + method.path + ' ' + method.path[:-9] + "after.rs " + repo.name + ' ' + commit.name + ' ' + vector_file)
                            

    print("corpus setup finished")
