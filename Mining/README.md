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

变量root_dir为生成改动代码的根目录

筛选条件：**改动条数在6行以内，commit message关键词**

### code/process.py

遍历search.py中生成的所有文件，对每个method生成对应的rust AST，文件排布和前者类似



### 下阶段任务

1、搞一个diff的工具，将改动前后的代码每行继续map对应起来

2、对改动前后对应的diff设计抽象，确定Change type和type类型（后续可以添加上下文信息），重点是先确定Change type，即Inserted/Removed/Updated

3、确定聚类的特征向量的维度





1、找10个5、6行修改的代码片段，用diffsitter测试

2、找其他几个工具，对比输出结果
