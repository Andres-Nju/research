import csv
import json
import numpy as np
from sklearn.cluster import DBSCAN
import matplotlib
matplotlib.use("webagg")
import matplotlib.pyplot as plt

commit_hash = {}
commit_cnt = 0
feature_cnt = 0
index_dic = {}
def process_commit_vector(vec_list: list):
    res = np.zeros(len(index_dic))
    #res = np.zeros(len(index_dic) + 1)
    #res[0] = commit_hash[(vec_list[0][0], vec_list[0][1])]
    for vec in vec_list:
        res[index_dic[vec[2], vec[3], vec[4]]] = res[index_dic[vec[2], vec[3], vec[4]]] + 1
    return res

def select_MinPts(data,k):
    k_dist = []
    for i in range(data.shape[0]):
        dist = ((np.abs(data[i] - data)).sum(axis=1))
        dist.sort()
        k_dist.append(dist[k])
    return np.array(k_dist)

if __name__ == '__main__':
    parent_dic = {}
    grandparent_dic = {}

    # 第一遍读，获取所有的commit数以及出现的节点类型
    with open("vector2.csv", "r", encoding="utf-8") as f:
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
            if (row[0], row[1]) not in commit_hash.keys():
                commit_hash[(row[0], row[1])] = commit_cnt
                commit_cnt = commit_cnt + 1

    # set the feature's index
    print(f'parent node number = {len(parent_dic)}')
    print(f'grandparent node number = {len(grandparent_dic)}')

    offset = len(parent_dic) * len(grandparent_dic)

    for i in sorted(parent_dic.items(), key = lambda x:x[1], reverse=True):
        for j in sorted(grandparent_dic.items(), key = lambda x:x[1], reverse=True):
            index_dic[("Added", i[0], j[0])] = feature_cnt
            index_dic[("Deleted", i[0], j[0])] = feature_cnt + offset
            feature_cnt = feature_cnt + 1



    
    print(f'commits nunber = {len(commit_hash)}')