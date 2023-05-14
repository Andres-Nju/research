### 一、Memory Access Reorder

- cpu多级指令流水线（以五级为例）

  - 同时执行
    - 取指令
    - 指令译码
    - 执行指令
    - 内存访问
    - 数据写回
  - 可以在一个时钟周期内同时执行五条指令的不同阶段

  ![](ref/1.png)

- 通过乱序执行，CPU可以在执行指令的同时，预先处理后续指令的执行，以避免因某个指令等待某个资源而导致的等待时间浪费。

```c
int A = 0;
int B = 0;

void fun() {
    A = B + 1; // L5
    B = 1; // L6
}

int main() {
    fun();
    return 0;
}
```

对应的汇编：

```assembly
movl    B(%rip), %eax
addl    $1, %eax
movl    %eax, A(%rip)
movl    $1, B(%rip)
```

g++ -O2 test.c生成的汇编：

```assembly
movl    B(%rip), %eax
movl    $1, B(%rip)
addl    $1, %eax
movl    %eax, A(%rip)
```

- 编译器只需要保证**在单线程环境下，执行的结果最终一致就可以**，所以，指令乱序在单线程环境下完全是允许的。对于编译器来说，它只知道：在当前线程中，数据的读写以及数据之间的依赖关系。

### 二、Introduction

- Rust中的类型系统使用一套严格的标准来限制**指针别名**，这种限制用来保证数据竞争等安全性，从而给指令重排的程序优化提供便利

```rust
fn example1 (x: & mut i32 , y: & mut i32 ) -> i32 {
  *x = 42;
  *y = 13;
  return *x; // Has to read 42, because x and y cannot alias !
}
```

Rust中，两个可变引用一定不是互为别名的关系，所以*x一定是42，那么return *x可以被优化成return 42；\*x和\*y的赋值语句可以随意调换位置。

但在别的语言中，这样的别名信息很难去获取：比如对于上述代码，换到C/C++中，无法确定x和y是否是别名关系。

- 但是，Rust的unsafe代码可以跳过借用规则

```rust
fn main () {
  let mut local = 5;
  let raw_pointer = & mut local as * mut i32;
  let result = unsafe { example1 (& mut * raw_pointer , & mut * raw_pointer ) };//cast *mut i32 back to &mut i32
  println !("{}", result ); // Prints "13".
}
```

​		

- 所以需要定义一套规则：只要用户遵守这套规则，即使指令重排的优化开启，也可以保证程序原本的语义不被改变。

- **Stacked Borrows——borrow checker的动态版本**

  - 定义了别名的规则，对于所有违背了该规则的程序，都会产生未定义行为。

  - 现有的Rust编译器语义规则下，上述样例代码将输出13 $\rightarrow$ 需要向语义规则中添加一些行为，告诉编译器它们是未定义行为；当然也不能添加太多——太严格就又变成safe Rust了

  - 静态检查的借用规则：

    - 同一时刻同一变量只能拥有1个可变引用或任意个不可变引用
    - 引用(reference)只能在其生命周期内被使用
    - 被引用的对象(referent)只能在loan的生命周期结束时被使用

    ```rust
    let mut v = vec ![10 , 11];
    let vptr = & mut v [1]; // Points * into * v. <-------------------------+
    v. push (12); // cannot borrow `v` as mutable more than once at a time  | lifetime of vptr and loan of v[1] 
    println !("v[1] = {}", * vptr );  <-------------------------------------+
    ```

    ![](ref/2.png)

  - 2、3两行reorder：

    ![](ref/3.png)

    或者说，同一个变量的reference的生命周期不能有重叠

  - **reborrow**：利用reference创建一个新的reference：嵌套生命周期

    ![](ref/4.png)

    如果在第五行后面再使用一次vptr，即lifetime 'b超出了'a的范围，那么就出现之前例子中一样的错误

  - **shared references**：不可变引用



- 动态程序分析：使用per-location stack，不使用borrow checker的lifetime
  - why not lifetime？
    - lifetime的推导一直在变化 old AST-based borrow checker $\rightarrow$ non-lexical lifetimes $\rightarrow$
    - 借用检查之后lifetime就没了，编译优化阶段无法获取这个信息。





### 三、Stacked Borrows —— built up incrementally

#### 1、考虑Rust中只有mutable reference的情况

- 静态borrow checker需要保证：

  - a reference and all references derived from it can only be used during its lifetime
  - 被引用的对象直到所有借用的生命周期消亡后才可以被使用

- 去掉lifetime这个概念，上面的规则可以写成：

  - 对于所有的引用（以及从该引用derived出来的引用），其**使用必须出现在被引用对象的下一次使用之前** —— 栈！！

  **stack principle **

  ```rust
  let mut local = 0;
  let x = & mut local ;
  let y = & mut *x; // Reborrow x to y.
  *x = 1; // Use x again .
  *y = 2; // Error ! y used after x got used .
  ```

  y的使用必须嵌套在x的两次使用中间

  定义-使用序列：xyxy不合法，xyyx合法

- 如何维护这样一个borrow stack？

  - 每当一个reference被创建时，将其push进来
  - 每当一个reference被使用后，它应该出现在栈顶；即不断对栈进行pop，直到该reference来到栈顶。
    - 对于被pop掉的reference——再也无法使用，如果使用就违背了规则



#### 2、Make the model more operational