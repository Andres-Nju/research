import json
import sys
import csv
if __name__ == '__main__':
    parent_dic = {}
    grandparent_dic = {}
    vec_file = sys.argv[1]
    # 第一遍读，获取所有的commit数以及出现的节点类型
    with open(vec_file, "r", encoding="utf-8") as f:
        reader = csv.reader(f)
        for row in reader: # repo, commit, added/deleted, parent, grandparent
            if row[3] not in parent_dic.keys():
                parent_dic[row[3]] = 1
            else:
                parent_dic[row[3]] = parent_dic[row[3]] + 1 

            if row[4] not in grandparent_dic.keys():
                grandparent_dic[row[4]] = 1
            else:
                grandparent_dic[row[4]] = grandparent_dic[row[4]] + 1 


    # set the feature's index
    print(f'parent node number = {len(parent_dic)}')
    print(f'grandparent node number = {len(grandparent_dic)}')

    for i in sorted(parent_dic.items(), key=lambda x: x[1], reverse=True):
        print(i)

    for i in sorted(grandparent_dic.items(), key=lambda x: x[1], reverse=True):
        print(i)