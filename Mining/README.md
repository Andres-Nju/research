### Lexing and Parsing

- **rustc_lexer**
  - source text $\rightarrow$ tokens
- **rustc_parse**
  - input : token stream (还无法进行parse) $(\rm rust\_parse::lexer)\rightarrow$ 可以被parse的token
  - **StringReader**
    - a set of validations
    - string interning

## Coding

### code/search.py 爬取仓库
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

​	变量root_dir为生成改动代码的根目录。

#### 筛选条件：

**改动条数在6行以内，**

**TODO: commit message关键词**



### code/gen_ast.py 

对前面获取的每个仓库改动代码文件，

使用code/get_tree进行parse，生成对应的AST并以相同的文件排版存储

变量root_dir为生成的AST的根目录。



```shell
code/get_tree/目录下：
cargo build
code/目录下：
python3 gen_ast.py
```



### code/get_tree 生成AST

- tree-sitter::parser::parse()：将源文件转化为tree-sitter::Tree
- tree-sitter::Tree.walk(): 获取TreeCursor用以遍历Tree



#### 生成的AST

**获取的tree-sitter::Tree节点类型：**

- 首先每个源文件中的内容都是一个function，所以Tree的最顶层的两个节点是
  - source_file
    - function_item
      - fn
      - identifier
      - parameters








### code/gen_diff.py 生成code diff信息

对于获取的每个仓库改动代码文件，使用difft指令生成改动前后代码的diff信息，并重定向写入文件。

```shell
code/目录下：
python3 gen_diff.py
```



### **difftsatic**

```shell
difft --display side-by-side-show-both --context 0 test1.rs test2.rs
```

- --display：显式模式：side-by-side-show-both：显式改动前后的对照

- --context：显式的源代码改动中包含的上下文行数

- 显示的内容：

  - 若前后代码没有语义上的区别，那么会显示：

    ```shell
    test2.rs --- Rust
    No syntactic changes.
    ```

    第一行是输入的后一个的文件路径与源代码所用语言的标识 --- Rust

    第二行是No syntactic changes的提示

  - 若前后代码只有一处改动（TODO：这里判断一处还是多处是由工具中的算法决定的），那么会有形如如下的显示：

    ```shell
    test2.rs --- Rust
    2     let b = 1;                                                     2 
    3                                                                    3 
    4     let s = String::from("23");                                    4     let s = String::from("123");
    .                                                                    5 
    .                                                                    6     let a = 2;
    .                                                                    7 
    5     if (b >= 0){                                                   8     if (b == 0 && b > 1){
    ```

    第一行和前面的一样，后面就是两个文件中的代码不同比较；

    TODO：如何判断是否是同一种变化？

    重定向后写入的文件中 同一行中前后都出现的代码表示前后的改动对应，其他的均为insert/remove
    
  - 若前后代码不止一处改动



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





