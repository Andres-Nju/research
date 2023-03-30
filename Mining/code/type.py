import json


if __name__ == '__main__':
    with open("./difftastic/vendored_parsers/tree-sitter-rust/src/node-types.json", 'r', encoding="utf-8") as f:
        content  = json.load(f)
        print(len(content))
        dic = {}
        for item in content:
            # print(item["type"])
            if "subtypes" in item.keys():
                '''for subtype_item in item["subtypes"]:
                    ty = subtype_item["type"]
                    if ty not in dic.keys():
                        dic[ty] = 1
                    else:
                        dic[ty] = dic[ty] + 1'''
                print("{} subtypes exist", item["type"])
            elif "field" in 
                # print(len(item["subtypes"]))
        # print(dic)