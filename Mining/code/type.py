import json


if __name__ == '__main__':
    with open("./difftastic/vendored_parsers/tree-sitter-rust/src/node-types.json", 'r', encoding="utf-8") as f:
        content  = json.load(f)
        print(len(content))
        dic = {}
        subtypes = {}
        for item in content:
            if "subtypes" in item.keys():
                for i in item["subtypes"]:
                    subtypes[i['type']] = 1
            if "fields" in item.keys() or "children" in item.keys():
                dic[item["type"]] = 1
        for key in dic.keys():
            print(key)
        print(len(dic.keys()))
        print(len(subtypes.keys()))