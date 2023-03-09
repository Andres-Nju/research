import json


if __name__ == '__main__':
    with open("./get_tree/tree-sitter-rust/src/node-types.json", 'r', encoding="utf-8") as f:
        content  = json.load(f)
        print(len(content))
        for item in content:
            print(item["type"])