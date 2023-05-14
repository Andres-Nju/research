- detected a total of 33 previously unknown bugs including 16 memory-safety issues from 12 Rust packages (crates) with an acceptable false-positive rate



- Why?
  - unsafe code is necessary for low-level operations such as dereferencing raw pointers
  - for pure safe Rust, sometimes to guarantee memory safety, program execution can be stopped.
    - 有些问题只能通过运行时动态检查出来，比如数组访问越界问题。But for some security-critical applications，运行时的crash是无法接受的
- how: 静态分析检测工具来检测 potential memory safety issues and runtime panics



 We argue that making a static analyzer dedicated for Rust by leveraging Rust’s type system has the following advantages: 

(1) Precision: 

Rust is statically and strongly typed, so the type system can provide more information to make the analysis more accessible and more precise. 

A dedicated analyzer can also take advantage of the **special patterns of bugs that are unique to Rust programs** (§ 3.1). 

(2) User-friendliness:

The Rust compiler **explicitly inserts assertions to check safety conditions dynamically** in order to prevent undefined behaviors. 

**These assertions can be used by the static analyzer as conditions** that should be checked (§ 4.2.3), thus no manual annotations are needed. 

(3) Efficiency: 

The ownership system statically determines the lifetime of each variable, so the analyzer can safely clean up the storage for variables that have gone out of their scopes. This dead variable cleaning mechanism reduces memory consumption and speeds up the analysis (§ 8.3).





- Introduction
  - based on theory of Abstract Interpretation
  - the analysis is done on the MIR
  - Core design follows the **monotone framework**
    - Transfer function defined for each statement an transferred by each statement
    - abstract domain gaters both numerical and symbolic values
      - 前者used for integer bounds analysis
      - 后者used as the memory model
  - 检测器会对现有的报告中的bug模式进行处理
  - contribution
    - a bug detector
    - a dedicated abstract domain that keeps track of both numerical and symbolic values in Rust programs
    - reveals 33 previously unknown bugs including 16memory-safefy issues.



- 本文主要聚焦两种bug
  - Runtime panics：Rust的类型系统不能在编译时期保证所有的security conditions，比如数组越界、整型数溢出、除零错误都要到运行时检查，虽然有panic机制避免了memory corruption，但是可能会导致denial-of-service attack的问题
    - According to a third-party bug collection repository trophy-case1 , about 40% of bugs are categorized as arithmetic error or out-of-range access.
  - Lifetime corruption：所有权机制在safe code中解决了传统的use-after-free、double free问题，但是unsafe代码的存在反而可能会让所有权机制出错
- solution
  - numerical static analysis计算每个整型变量的边界
  - symbolic static analysis来追踪内存区的所有权，特别关注一些可能导致别名问题的unsafe函数 





- 两个例子
  - 整型数溢出
  - use-after-free
- Architecture
  - User interface
    - MirChecker can be activated by a subcommond of Cargo
  - static analyzer
  - bug detection based on the results of the analyzer
    - 2 categories of security conditions
      - runtime assertions
      - common memory-safety error patterns



- Language Model
  - 赋值语句p = r
    - 左边Place的p可能是
      - 变量
      - field （p.n）
      - 指针解引用（*p）
      - 数组的下标访问（p[v]）
    - 右边是一个右值
      - 可能是数值运算
      - 比较运算
      - 逻辑反
      - 取负
      - 引用（&p）
      - 类型转换（p as ...）
    - RUst MIR会将左值和右值区分开，并且类型系统会保证p和r是相同数据类型，三地址码保证不会有嵌套的情况
  - 二元运算操作
    - 每个操作数都可能是一个constant或place
    - 这两个操作数和结果都是整型数
  - 比较运算
    - 生成的结果不是整型而是boolean
  - 函数调用
    - 函数f
    - 一系列参数
    - 返回值p在block b中
  - Drop(p)
  - 断言Assert(op)
  - 跳转Goto(b)
  - Switch语句SwitchInt
    - op为整型值
- Memory Model：describe how the analyzer should handle the memory operations when accessing the memory
  - quite difficult to decide the precise memory cell，比如内存读写的操作，具体的内存地址可能和程序的输入有关，而且通常是动态运行时确定的。
  - 传统方法是指针分析，但是指针分析主要针对low-level的ir，其memory model通常只有简单的load/store，但是rust MIR有更复杂的Memory access范式—— place expression
    - 所以每当获取一个Place时，构造一个符号表达式来当作它的抽象地址；再细化就是维护一张memory lookup table，这个表达式用作查找表的key
    - 如何判断两个抽象地址是相同的就变成判断对应的符号表达式是否相同
- Abstract values
  - 为每个block维护一个P->V的查找表，其中P代表所有Place的集合，V表示Place可能指向的所有抽象值
    - 两个特殊的抽象值：bottom表示为初始化的值，top表示所有可能的值
    - 集合V又由两部分组成：数值NV和符号值SV，数值用来求每个整型变量的边界，符号值用来表示其抽象内存地址和控制流分支情况
  - 即每个程序点处的程序状态为P->V的一个map，那么定义程序状态的格为AS，其元素是有P->V的maps构成，abstract domain即AD为B->AS的map，即维护每个block上的AS
- transfer function
  - 根据前面定义的language model来制定转换规则，比如赋值语句，就是到查找表中找到赋值语句的左值对应的抽象值，然后用右值更新（数值和符号信息同时更新）
- 如何提取出抽象值？或者说如何判断抽象值是在数值domain上更新还是在符号domain上更新
  - 首先数值更新一定是一下三种语句：
    - 赋值语句
    - 二元运算语句
    - 取负语句
  - 并且对于赋值语句有一定限制：右值是一个整型变量
  - 对于符号域，包括以下两个内容
    - 抽象内存地址
    - 分枝判断条件
  - 用了一些归约规则来简化符号分析的内容，并且在这个归约的过程中，数值分析的结果能派上用场。比如判断语句的两个操作数的数值分析结果可以推出判断语句永真/假，那么就可以直接将左值赋上对应的bool值，而不是加入整和比较语句；再者，如果简化后的语句结果是一个整型，那么可以直接加入数值域
  - numerical和symbolic两种分析可以相互促进
- 算法
  - 遍历CFG时，如果遇到循环，那么会一直遍历直到该循环达到不动点
  - 每个basic block的最后，都加上一个statement叫做terminator，指明控制流的流向，比如条件分支使用SwitchInt terminator来表示
  - if cond {...} else {...}可以添加一些符号约束，可能在数值分析中可以用到
- MIR中StorageLive和StorageDead语句可以帮助消除分析中的死变量
- 误报抑制
  - 误报的原因
    - 静态分析不可避免
    - 有些程序员故意加进去的panic
    - 一个bug可能在不同执行路径上被处罚，然后导致多处的错误报告