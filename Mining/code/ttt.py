from pydriller import Repository
import os
import sys
class MyMethod:
    def __init__(self, name, start_line_before, end_line_before, start_line_changed, end_line_changed) -> None:
        self.name = name
        self.start_line_before = start_line_before
        self.end_line_before = end_line_before
        self.start_line_changed = start_line_changed
        self.end_line_changed = end_line_changed

def filter_methods(methods_before, changed_methods):
    res = []
    candidate = {}
    for changed_method in changed_methods:
        for method_before in methods_before:
            if method_before.name == changed_method.name:
                if changed_method not in candidate.keys():
                    candidate[changed_method] = [(method_before.start_line, method_before.end_line)]
                else:
                    candidate[changed_method].append((method_before.start_line, method_before.end_line))
    for key in candidate.keys():
        if len(candidate[key]) == 1:    
            res.append(MyMethod(key.name, candidate[key][0][0], candidate[key][0][1], key.start_line, key.end_line))
        else:
            t = (0, 0)
            distance = sys.maxsize
            for tuple in candidate[key]:
                cur_dis = abs(key.start_line - tuple[0])
                if cur_dis < distance:
                    distance = cur_dis
                    t = tuple
            res.append(MyMethod(key.name, t[0], t[1], key.start_line, key.end_line))
    return res


if __name__ == '__main__':
    cur_path = os.getcwd()
    for commit in Repository(cur_path + "/Commits/alacritty", only_modifications_with_file_types=['.rs'], only_commits=["1d949d72d4dcf3b569c85ebdbc7b252dcf8b9a2e"]).traverse_commits():
        for modified_file in commit.modified_files:
            # res = filter_methods(modified_file.methods_before, modified_file.changed_methods)
            for method1 in modified_file.changed_methods:
                for method2 in modified_file.methods_before:
                    if method1.long_name == method2.long_name:
                        print(method1.long_name)
            
            
        