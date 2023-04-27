import csv
import json
import sys
import numpy as np
import sklearn.cluster as clusters
import matplotlib
matplotlib.use("webagg")
import matplotlib.pyplot as plt
import pandas as pd

commits = []
commit_hash = {}
index_dic = {}
commit_cnt = 0
feature_cnt = 0

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

def get_clusters(labels, commits):
    res = {}
    for index, value in enumerate(labels):
        if value != -1:
            if value not in res.keys():
                res[value] = [commits[index]]
            else:
                res[value].append(commits[index])
    return res

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
            if (row[0], row[1]) not in commit_hash.keys():
                commit_hash[(row[0], row[1])] = commit_cnt
                commit_cnt = commit_cnt + 1
                commits.append((row[0], row[1]))

    # set the feature's index
    print(f'parent node number = {len(parent_dic)}')
    print(f'grandparent node number = {len(grandparent_dic)}')

    offset = len(parent_dic) * len(grandparent_dic)

 

    ''' # 构建空列表，用于保存不同参数组合下的结果
    # DB-SCAN
    res = []
    # 迭代不同的eps值
    for eps in np.arange(1,30,1):
        # 迭代不同的min_samples值
        for min_samples in range(1,3):
            dbscan = clusters.DBSCAN(eps=eps, min_samples=min_samples, metric="manhattan")
            # 模型拟合
            dbscan.fit(vecs)
            # 统计各参数组合下的聚类个数（-1表示异常点）
            n_clusters = len([i for i in set(dbscan.labels_) if i != -1])
            # 异常点的个数
            outliners = np.sum(np.where(dbscan.labels_ == -1, 1,0))
            # 统计每个簇的样本个数
            stats = str(pd.Series([i for i in dbscan.labels_ if i != -1]).value_counts().values)
            res.append({'eps':eps,'min_samples':min_samples,'n_clusters':n_clusters,'outliners':outliners,'stats':stats})
    # 将迭代后的结果存储到数据框中        
    df = pd.DataFrame(res)

    # 根据条件筛选合理的参数组合
    print(df)'''



    ''' # find the best eps
    k = 4
    k_dist = select_MinPts(vecs,k)
    k_dist.sort()
    #plt.plot(np.arange(k_dist.shape[0]),k_dist[::-1])
    eps = k_dist[::-1][8]
    #plt.scatter(8,eps,color="r")
    #plt.plot([0,8],[eps,eps],linestyle="--",color = "r")
    #plt.plot([8,8],[0,eps],linestyle="--",color = "r")
    #plt.show()
    print(eps)


    dbscan_model = DBSCAN(eps=eps,min_samples=k+1,metric="manhattan")
    label = dbscan_model.fit_predict(vecs)
    print(label)'''

    '''d = {}
    for key1 in parent_dic.keys():
        d[key1] = 1
    for key2 in grandparent_dic.keys():
        d[key2] = 1'''


    ''' with open("./difftastic/vendored_parsers/tree-sitter-rust/src/node-types.json", 'r', encoding="utf-8") as f:
        content  = json.load(f)
        print(len(content))
        dic = {}
        subtypes = {}
        super_type = {}
        for item in content:
            if "subtypes" in item.keys():
                super_type[item['type']] = 1
                for i in item["subtypes"]:
                    subtypes[i['type']] = 1
            if "fields" in item.keys() or "children" in item.keys(): 
                if ("fields" in item.keys() and len(item['fields']) > 0) or ("children" in item.keys() and len(item['children']) > 0) :
                    dic[item["type"]] = 1
        for key in dic.keys():
            print(key)
        print(f'super type的子节点个数: {len(subtypes.keys())}') 

        print(f'vector中出现的节点个数 包括父节点和祖父节点{len(d.keys())}')
        print(f'有孩子的节点个数{len(dic.keys())}')
        for key in dic.keys():
            if key not in d.keys():
                print(key)'''

    
    print(f'commits nunber = {len(commit_hash)}')