### Lexing and Parsing

- **rustc_lexer**
  - source text $\rightarrow$ tokens
- **rustc_parse**
  - input : token stream (还无法进行parse) $(\rm rust\_parse::lexer)\rightarrow$ 可以被parse的token
  - **StringReader**
    - a set of validations
    - string interning



### code/search.py
```shell
python3 search.py repos.txt
```

使用如上指令，将遍历repos.txt中的所有仓库

获取仓库改动流程：

1. 读取仓库名
2. 遍历仓库中与后缀名为.rs（即rust源代码文件）的commit
3. 遍历每个commit中改动的.rs文件 （过滤其它类型的文件），并过滤掉改动前/改动后的文件缺失的情况（即文件级别的add/delete的情况）
4. 获取改动前后的method信息（名称、在源文件中的起始行与结束行），并过滤出**在文件改动前后都出现**的method（即过滤掉method被add/delete的情况，仅保留modify）
5. 获取到改动前后的method源代码，并存入文件，文件排布为 仓库——commit——改动文件——method\_改动前、method\_改动后

​	变量root_dir为生成改动代码的根目录

#### 筛选条件：

**改动条数在6行以内，**

**TODO: commit message关键词**



### code/get_tree

- tree-sitter::parser::parse()：将源文件转化为tree-sitter::Tree
- tree-sitter::Tree.walk(): 获取TreeCursor用以遍历Tree



#### 生成的AST

**获取的tree-sitter::Tree节点类型：**

- 首先每个源文件

### **difftsatic**

```shell
difft --display side-by-side-show-both --context 0 test1.rs test2.rs
```

- --display：显式模式：side-by-side-show-both：显式改动前后的对照
- --context：显式的源代码改动中包含的上下文行数



### 特征向量抽象：

- change type：
  - insert：使用difftsatic显示的前后改动中 左边没有，右边有
  - delete：使用difftsatic显示的前后改动中 左边有，右边没有
  - update：使用difftsatic显示的前后改动中 左边有，右边也有
- context：
  - 使用TreeCursor在tree-sitter::Tree中遍历







## 跑起来可能会crash的点：

- pydriller遍历commit崩了
- tree-sitter::parse语法分析器解析失败

#### 下周任务：确定context中的节点





