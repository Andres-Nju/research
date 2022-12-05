## 语言特性

- variable : immutable by default 

  - Mut

  - shadow

    ```rust
    let a = 1
    
    {
      let a = "hello" 
    }
    ```

  - type: 

    - integer overflow --> 

      - debug pattern : program panic
    - release pattern : not check, but two’s complement wrapping (e.g. 1 0000 0001 --> 0000 0001)

    - bool : if语句中的条件语句只能是bool type

      ```rust
      if num{//num must be bool type
      	...
      }
      //or:
      if num != 0{
        ...
      }
      ```

      

- Functions : consisting of statements and ending with an expression —— Rust's **expression-based** attribution

  - statements end with a semicolon (';'), while expressions don't

  - expressions have return values, while statements don't 

    ```rust
    fn main() {
        let x = 5;
    
        let y = {
            let x = 3;
            x + 1
        };
    
        println!("The value of y is: {}", y);
    }
    
    //其中
     {
            let x = 3;
            x + 1
      } 也是个表达式，返回的值是x + 1
    
    C中可以出现x = y = 6这样的语句，即赋值语句(assigning statement)也有返回值，但rust中不可以
    //if 语句也可以带返回值
    let number = if condition {
            5
        } else {
            6
        };//和前面的类似
    ```

- 字符串常量和slice：都是&str类型

- memory management of Rust

  - traditional memory management system:
    - garbage collection system
    - free manually
  - ownership-based resource management system
    - each variable has an 'ownership' correspond with its allocated memory, parameter passing and value returning will "move" the ownership
  - two mutex kind of trait
    - copy trait for stack-only data : if rvalue has copy trait, Rust will copy it in the stack and the older variable is still usable.
    - drop trait for others (heap, ...): if rvalue has drop trait, Rust will move the ownership from it to lvalue and the older variable will be no longer available. 
  - automatic RAII, i.e. resources are bounded with valid scopes (particularly, a drop scope for each drop-trait variable, thus when an initialized variable goes out of the drop scope, the destructor will recursively drop its fields)
    - free resources without garbage collection —— automatically run destructors (drop the pointers) once the control-flow leaves the drop scope 

- basics of Rust compiler

  - detecting memory-safety violations is based on MIR

```rust
BasicBlock := {Statement} Terminator
Statement := LValue = RValue | StorageLive(Value)
		| StorageDead(Value) | ...
LValue := LValue | LValue.f | *LValue | ...
RValue := LValue | move LValue
		| & LValue | & mut LValue
		| * LValue | * mut LValue
		| ...
Terminator := Goto(BB) | Panic(BB)
		| Return | Resume | Abort
		| If(Value, BB0, BB1)
		| LVALUE = (FnCall, BB0, BB1)
		| Drop(Value, BB0, BB1)
		| SwitchInt(Value, BB0, BB1, BB2, ...)
		| ...
```

- borrow checker of Rust

  - basic idea: a value may not be mutated or moved while it is being borrowed

    - ---> question: how to know whether a value is being borrowed?

      whenever create a borrow (i.e. a reference) of a variable, give the reference a lifetime

    - lifetime of

      - reference: correspond to the time in which the reference is used

      - value: the time before it is freed, i.e. its scope 

        Obviously, lifetime of the reference may not outlive the scope of its referent

        ```rust
        fn foo() {
            let mut data = vec!['a', 'b', 'c']; // --+ 'scope
            capitalize(&mut data[..]);          //   |
        //  ^~~~~~~~~~~~~~~~~~~~~~~~~ 'lifetime //   |
            data.push('d');                     //   |
            data.push('e');                     //   |
            data.push('f');                     //   |
        } // <---------------------------------------+
        
        fn capitalize(data: &mut [char]) {
            // do something
        }
        ```

        the lifetime of the reference is just within single statement, i.e. confined to the call to function capitalize(), thus data can be mutated in the next statement.

      - Problem arises when the lifetime spans multiple statements (compiler often regards it as a block, which is much bigger than what is really desired)

        ```rust
        fn bar() {
            let mut data = vec!['a', 'b', 'c'];
            let slice = &mut data[..]; // <-+ 'lifetime
            capitalize(slice);         //   |
            data.push('d'); // ERROR!  //   |
            data.push('e'); // ERROR!  //   |
            data.push('f'); // ERROR!  //   |
        } // <------------------------------+
        ```

        the `&mut data[..]` slice is not passed directly to `capitalize`, but is instead stored into a local variable

        —— assigning a reference to a variable makes the lifetime the same as the scope of the variable, which is logical but annoying

        ```rust
        fn bar() {
            let mut data = vec!['a', 'b', 'c'];
            {
                let slice = &mut data[..]; // <-+ 'lifetime
                capitalize(slice);         //   |
            } // <------------------------------+
            data.push('d'); // OK
            data.push('e'); // OK
            data.push('f'); // OK
        }
        ```

        So introduce a new block to limit the lifetime of reference within a smaller scope

        but it is certainly not convenient enough

  - borrow checker based on lifetime inference

    - NLL Non-lexical lifetime

- struct

  - 只有整个实例被声明为mutable时，其field才是mutable的，不允许只将某个字段设置为mutable

  - 通过为每个字段指定具体值来创建这个结构体的 **实例**，构造的时候不要求域的顺序和定义的顺序一致

    ```rust
    struct User {
        username: String,
        email: String,
        sign_in_count: u64,
        active: bool,
    }
    
    let user1 = User {
        email: String::from("someone@example.com"),
        username: String::from("someusername123"),
        active: true,
        sign_in_count: 1,
    };
    //可以用某个函数构造结构体实例并返回
    
    ```
    
    发现email和username是同名的，重复一遍有些麻烦，使用**字段初始化简写语法**，即当参数名和字段名相同时，可以省略
    
    ```rust
    fn build_user(email: String, username: String) -> User {
        User {
            email,
          username,
            active: true,
          sign_in_count: 1,
        }
    }
    ```
    
    当创建一个新的实例需要用到另一个实例的部分值时：
    
    ```rust
    # let user1 = User {
    #     email: String::from("someone@example.com"),
    #     username: String::from("someusername123"),
  #     active: true,
    #     sign_in_count: 1,
  # };
    #
    let user2 = User {
        email: String::from("another@example.com"),
        username: String::from("anotherusername567"),
        active: user1.active,
        sign_in_count: user1.sign_in_count,
    };

    ```
    
    或者使用**结构体更新语法**，创建一个user2实体，用到user1的部分值
    
    ```rust
    let user2 = User {
        email: String::from("another@example.com"),
        username: String::from("anotherusername567"),
      ..user1
    };
  //..表示剩余的未显示赋值的字段都使用与user1的相应字段的相同的值
    ```

  - 获取实例其中某个field，使用object.field

  - 定义元组结构体：没有具体的字段名，只有字段的类型，以struct关键字和结构体名开头，后面括号内跟元组中字段的类型即可

    - 将tuple从一个variable/value变成一个type

  ```rust
    struct Color(i32, i32, i32);
  struct Point(i32, i32, i32);
    
  let black = Color(0, 0, 0);
    let origin = Point(0, 0, 0);
  //其中black和origin不是同一类型，即使Color和Point类的字段类型完全相同
    //？那么如果两个结构体的字段类型相同呢
  ```

  - 结构体的解构（destruct）：使用let

    ```rust
    //元组的解构：
    let tup = (500, 6.4, 1);
    let (x, y, z) = tup; //x = 500, y = 6.4, z = 1

    //元组结构体的解构
  struct Point(i32, i32, i32);
    
    let point = Point(1, 2, 3);//创建一个元组结构体的实例
    let Point (x, y, z) = point; //x = 1, y = 2, z = 3
    
    //结构体的解构
    struct Point{
      x: f32,
      y: f32,
      z: f32,
    }
    let point = Point {x: 1.1, y: 2.2, z: 3.3};//创建一个结构体的实例
    let Point {x: a, y: b, z: c} = point;//a = 1.1, b = 2.2, c = 3.3
    ```

  - 可以在结构体的的上下文中（impl关键字）定义方法

    ```rust
    struct Rectangle {
        width: u32,
        height: u32,
    }
    

  impl Rectangle {
        fn area(&self) -> u32 {
            self.width * self.height
        }
    }
    fn main() {
        let rect1 = Rectangle { width: 30, height: 50 };

        println!(
            "The area of the rectangle is {} square pixels.",
            rect1.area()
        );
    }
    ```
  
    &self表示该方法并不想获取实例的所有权，且这里只需要读，不需要写
  
    但是会发现，这里rect1.area()调用时，其实应该是(&rect1).area()才符合函数签名，所以Rust中有一个 **自动引用和解引用（autoatic referencing and dereferencing）**，即当使用类似object.something()调用方法时，Rust会自动为object添加&/&mut/*来使得object与方法签名匹配
  
  - 允许在impl中定义不以self为参数的函数（**关联函数（associated function）**），与结构体相关联，但不是方法而是函数，因为并不作用于一个结构体的实例
  
  - 通常用作返回一个结构体新实例的构造函数
    - 使用结构体名::函数名来调用这个关联函数（类似C++类中的static函数）
  
  - **类单元结构体**（*unit-like structs*）没有任何字段的结构体 第十章
  
  - 结构体数据的所有权？是否Drop trait还是copy trait

- 枚举

  ```rust
  enum IpAddrKind{
  	V4,
    V6,
  }
  let four = IpAddrKind::V4;
  let six = IpAddrKind::V6;
  //枚举类型可以作为参数传入函数
  fn route(ip_type: IpAddrKind) {...}
  route(four);
  route(six);
  ```

  - 可以将数据放入每一个枚举类型的成员

    ```rust
    //本来一个ip对应一个类型（V4/V6）和一个地址
    struct IpAddr{
      kind: IpAddrKind,
      address: String,
    }
    let home = IpAddr{
      kind: IpAddrKind::V4,//是否可以用four?
      address: String::from("127.0.0.1");
    }
    //将数据和成员绑定，可以实现和上面结构体一样的功能
    enum IpAddr{
      V4(String),
      V6(String),
    }
    let home = IpAddr::V4(String::from("127.0.0.1"));
    let loopback = IpAddr::V4(String::from("::1"));
    ```

    有一个很明显的好处：比如想把ipV4的地址类型从String改成4个u8类型的数，而ipV6的地址类型还是String

    ```rust
    enum IpAddr{
      V4(u8, u8, u8, u8),
      V6(String),
    }
    let home = IpAddr::V4(127, 0, 0, 1);
    let loopback = IpAddr::V4(String::from("::1"));
    ```

  - 可以将任意类型的数据放入枚举成员！（字符串，数值类型，结构体...）甚至是一个枚举也可以

    ```rust
    enum WebEvent {
        // An `enum` may either be `unit-like`,
        PageLoad,
        PageUnload,
        // like tuple structs,
        KeyPress(char),
        Paste(String),
        // or c-like structures.
        Click { x: i64, y: i64 },
    }
    //将enum作为参数传入：
    fn inspect(event: WebEvent) {
        match event {
            WebEvent::PageLoad => println!("page loaded"),
            WebEvent::PageUnload => println!("page unloaded"),
            // Destructure `c` from inside the `enum`.
            WebEvent::KeyPress(c) => println!("pressed '{}'.", c),
            WebEvent::Paste(s) => println!("pasted \"{}\".", s),
            // Destructure `Click` into `x` and `y`.
            WebEvent::Click { x, y } => {
                println!("clicked at x={}, y={}.", x, y);
            },
        }
    }
    //enum也可以在impl中定义方法
    impl WebEvent{
      fn inspect(&self) {
        match self {
            WebEvent::PageLoad => println!("page loaded"),
            WebEvent::PageUnload => println!("page unloaded"),
            // Destructure `c` from inside the `enum`.
            WebEvent::KeyPress(c) => println!("pressed '{}'.", c),
            WebEvent::Paste(s) => println!("pasted \"{}\".", s),
            // Destructure `Click` into `x` and `y`.
            WebEvent::Click { x, y } => {
                println!("clicked at x={}, y={}.", x, y);
            },
        }
    	}
    }
    fn main(){
      let pressed = WebEvent::KeyPress('x');
      pressed.inspect();
    }
    ```

    这样一个enum有点像把这么多个结构体全部组合到一个enum类型下，但是可以这些不同的内容可以作为一个枚举类型的参数传入一个函数

  - Option类型

    ```rust
    enum Option {
        Some(T),
        None,
    }
    
    let some_number = Some(5);
    let some_string = Some("a string");
    
    let absent_number: Option<i32> = None;
    ```

    这里的Option和T并不是同一类型，比如：

    ```rust
    let x: i8 = 5;
    let y: Option<i8> = Some(5);
    
    let sum = x + y;//error
    ```

    如果想要用some()中的内容，需要将其unwrap

- match控制流运算符：类似switch语句，将一个值和一系列模式进行比较并根据匹配的情况执行后续代码。

  - 模式可以由字面值、变量、通配符和许多其他内容组成

  - 比如一个以枚举类型的变量和枚举成员作为模式的match：

    ```rust
    enum Coin {
        Penny,
        Nickel,
        Dime,
        Quarter,
    }
    
    fn value_in_cents(coin: Coin) -> u32 {
        match coin {//每一个分支之间用逗号间隔
            Coin::Penny => 1,
            Coin::Nickel => 5,
            Coin::Dime => 10,
            Coin::Quarter => 25,
        }
    }
    ```

  - 每一个分支之间用逗号间隔，每个分支相关联的代码是一个表达式，表达式的结果值将作为整个match表达式的返回值

    - 如果分支中需要执行多行代码，可以使用大括号

  - match的另一个有用的功能是可以**绑定匹配的模式的部分值**，这也是如何从枚举成员中提取值的

    ```rust
    # #[derive(Debug)]
    # enum UsState {
    #    Alabama,
    #    Alaska,
    # }
    #
    # enum Coin {
    #    Penny,
    #    Nickel,
    #    Dime,
    #    Quarter(UsState),
    # }
    #
    fn value_in_cents(coin: Coin) -> u32 {
        match coin {
            Coin::Penny => 1,
            Coin::Nickel => 5,
            Coin::Dime => 10,
            Coin::Quarter(state) => {
                println!("State quarter from {:?}!", state);
                25
            },
        }
    }
    //state会获取Quarter所绑定的UsState的具体值
    ```

  - 匹配是穷尽的！举个例子，比如匹配Option，代码中只写了匹配到some(T)的情况而没有写None的处理，那么编译会报错

  - '_'通配符：放在所有的分支之后，会匹配剩下的所有可能的值，比如匹配一个0-255内的任意值，我们只关心1，2，3，4这四个值的处理，那么其他的值就可以用\_代替。

    - 当match只需要关心**一个**情况的时候，就可以用到if let来代替了

    ```rust
    let some_u8_value = Some(0u8);
    match some_u8_value {
        Some(3) => println!("three"),
        _ => (),
    }
    ```

    只关心Some(3)的情况，对其他的Some(T)和None的情况不做操作。为了满足match的穷尽性，需要在后面加上_=>()的处理。

    然而使用if let就很方便，即当值匹配某一模式时执行代码而忽略所有其他值

    ```rust
    let some_u8_value = Some(0u8);
    if let Some(3) = some_u8_value {
        println!("three");
    }
    ```

  - 同时，if let可以加上else，else中的代码等于match中通配符\_分支中的代码
  
- 包

  - 带有Cargo.toml的是一个包，用来描述如何构建一个/多个crate，一个包中至多有一个库项目
  - crate是一个二进制项目/库项目
  - crate根决定了该crate是什么项目
    - src/main.rs：二进制项目
    - src/lib.rs：库项目
    - 如果两个都有，那么该包有两个crate：一个二进制项目和一个库项目
  - 包可以带有多个二进制crate，需要将其文件置于src/bin目录下，其中每个文件将是一个单独的crate

- 模块

  - 模块可以嵌套

    ```rust
    mod sound {
        mod instrument {
            mod woodwind {
                fn clarinet() {
                    // 函数体
                }
            }
        }
    
        mod voice {
    
        }
    }
    
    fn main() {
    
    }
    //the module tree:
    crate
    └── sound
        ├── instrument
        │   └── woodwind
        └── voice
    ```

  - 使用**路径**来饮用模块树中的项（比如想要调用某个模块中的函数，那么就要先知道其路径）

    - 绝对路径：从crate根开始，以crate名或字面值“crate”开头
    - 相对路径：从当前模块开始，以self、super或当前模块的标识符开头

    上述例子中在main函数中调用clarinet()函数（将样例代码简化了一下）：

    ```rust
    mod sound {
        mod instrument {
            fn clarinet() {
                // 函数体
            }
        }
    }
    
    fn main() {
        // 绝对路径
        crate::sound::instrument::clarinet();
    
        // 相对路径
        sound::instrument::clarinet();
    }
    ```

  - 会出现性问题：编译器会说instrument模块是**私有**的，模块在Rust中是**私有性边界**，即当想要让函数或结构体成为私有的，就可以将其放入模块。私有性规则如下：

    - 所有项（函数、方法、结构体、枚举、模块和常量）都是私有的
    - 可以使用“**pub**”关键字使项变为公有
    - 不允许使用定义于当前模块的子模块中的私有代码（外层不能访问内层）
    - 允许使用任何定义于父模块或当前模块中的代码（内层可以访问外层和当前层）

  - 想要刚才的代码中能够访问到instrument，就需要将其变为公有

    ```rust
    mod sound {
        pub mod instrument {
            fn clarinet() {
                // 函数体
            }
        }
    }
    
    fn main() {
        // Absolute path
        crate::sound::instrument::clarinet();
    
        // Relative path
        sound::instrument::clarinet();
    }
    ```

  - 但是，在模块前添加pub关键字只能使模块变为公有，但**模块内容**仍是私有的，所以上面还是不能访问clarinet()函数

    ```rust
    mod sound {
        pub mod instrument {
            pub fn clarinet() {
                // 函数体
            }
        }
    }
    
    fn main() {
        // 绝对路径
        crate::sound::instrument::clarinet();
    
        // 相对路径
        sound::instrument::clarinet();
    }
    ```

    需要将clarinet函数也定义为pub的

  - 为什么sound模块不需要pub关键字？

    - 因为main函数和sound模块是定义在同一个crate根模块中的，所以可以在main中引用sound模块

  - super关键字：super代表路径从**父**模块开始，相当于文件系统中的..

    ```rust
    # fn main() {}
    #
    mod instrument {
        fn clarinet() {
            super::breathe_in();
        }
    }
    
    fn breathe_in() {
        // 函数体
    }
    ```

    例子中通过super关键字访问instrument的父模块中的breath_in函数

  - 同样的，可以将结构体和枚举用pub关键字设置为公有

    - 在结构体定义中使用pub，使得结构体公有，但是结构体的**字段**仍是私有的，但可以在每一个字段上设置是否共有

      ```rust
      mod plant {
          pub struct Vegetable {
              pub name: String,
              id: i32,
          }
      
          impl Vegetable {
              pub fn new(name: &str) -> Vegetable {
                  Vegetable {
                      name: String::from(name),
                      id: 1,
                  }
              }
          }
      }
      
      fn main() {
          let mut v = plant::Vegetable::new("squash");
      
          v.name = String::from("butternut squash");
          println!("{} are delicious", v.name);
      
          // 如果将如下行取消注释代码将无法编译:
          // println!("The ID is {}", v.id);
      }
      ```

      注意，由于plant::Vegetable拥有一个私有字段Vegetable.id，所以无法在main中通过直接设置id的值来构建Vegetable的实例，就需要提供一个公有的关联函数来构建Vegetable的实例

    - 但是对于枚举，当一个枚举为公有时，其所有成员都是公有的，即只需要在enum关键词前加上pub，就可以访问所有成员

      ```rust
      mod menu {
          pub enum Appetizer {
              Soup,
              Salad,
          }
      }
      
      fn main() {
          let order1 = menu::Appetizer::Soup;
          let order2 = menu::Appetizer::Salad;
      }
      ```

  - 使用**use**关键字将名称引入作用域

    - 上面pub模块的例子中可以发现，每次访问clarinet都需要加上一大串前缀来指明路径，所以使用use关键字，能够一次性将路径引入作用域

    ```rust
    mod sound {
        pub mod instrument {
            pub fn clarinet() {
                // 函数体
            }
        }
    }
    
    use crate::sound::instrument;
    
    fn main() {
        instrument::clarinet();
        instrument::clarinet();
        instrument::clarinet();
    }
    ```

    这里use用的是绝对路径

    也可以使用相对路径来use，就要用到self关键字来代替当前路径

    ```rust
    mod sound {
        pub mod instrument {
            pub fn clarinet() {
                // 函数体
            }
        }
    }
    
    use self::sound::instrument;
    
    fn main() {
        instrument::clarinet();
        instrument::clarinet();
        instrument::clarinet();
    }
    ```

  - Rust中不推荐比如直接use ...::instrument::clarinet然后可以直接在main函数中clarinet()来调用该函数，习惯用法是use将该函数的父模块即instrument引入作用域，然后根据该父模块来调用该函数。因为指定父模块可以表明该函数不是本地定义的

  - 对于结构体、枚举和其他项，习惯使用use指定项的全路径

    ```rust
    use std::collections::HashMap;
    
    fn main() {
        let mut map = HashMap::new();
        map.insert(1, 2);
    }
    ```

  - 但是，当要引入两个不同模块中的同名项时，只能引入它们各自的父模块，然后通过父模块来访问不同的同名项

  - **as**关键字重命名引入作用域的类型

    - 当引入不同模块的同名项是，还可以通过在use后加上as和一个新名称来给该类型指定一个新的本地名称

      ```rust
      use std::fmt::Result;
      use std::io::Result as IoResult;
      
      fn function1() -> Result {
      #     Ok(())
      }
      fn function2() -> IoResult<()> {
      #     Ok(())
      }
      ```

  - **pub use**关键字**重导出**

    使用use关键字将名称导入作用域A时，在其他的作用域中该名称依旧是A中私有的，如果希望在任何作用域中都可以使用，可以将pub和use结合起来

    ```rust
    mod sound {
        pub mod instrument {
            pub fn clarinet() {
                // 函数体
            }
        }
    }
    
    mod performance_group {
        pub use crate::sound::instrument;
    
        pub fn clarinet_trio() {
            instrument::clarinet();
            instrument::clarinet();
            instrument::clarinet();
        }
    }
    
    fn main() {
        performance_group::clarinet_trio();
        performance_group::instrument::clarinet();
    }
    ```

    在performance_group模块中引入了instrument，所以可以调用instrument::clarinet()，但main函数在和performance_group同一层的另一模块中，可以调用performance_group下的所有内容，除了引入的instrument，所以使用pub use后，才可以在main中直接调用instrument中的内容

  - 使用外部包

    - 在Cargo.toml中加入依赖
    - 在项目中use引入

  - 需要引入很多相同包或模块的项时，可以使用嵌套路径来消除大量的use行

    ```rust
    use std::cmp::Ordering;
    use std::io;
    // ---snip---
    ==>
    use std::{cmp::Ordering, io};
    // ---snip---
    
    use std::io;
    use std::io::Write;
    ==>
    use std::io::{self, Write};
    ```

  - 使用glob运算符将一个路径下所有的公有项引入

    ```rust
    use std::collections::*;
    ```

  - 将模块分割到不同文件

    ```rust
    //main.rs
    mod sound;
    
    fn main() {
        // 绝对路径
        crate::sound::instrument::clarinet();
    
        // 相对路径
        sound::instrument::clarinet();
    }
    
    //sound.rs
    pub mod instrument {
        pub fn clarinet() {
            // 函数体
        }
    }
    ```

    将sound模块的定义放入sound.rs文件中，然后在crate根即main.rs中声明mod sound; 直接用分号结尾，告诉Rust该模块定义在同名的.rs文件中

- 集合

  - Vector

    新建一个空的vector

    ```rust
    let v: Vec<i32> = Vec::new();//需要加上类型注解
    //但是如果直接用初始值来创建，就不需要加上类型注解
    let v = vec![1, 2, 3]//不需要加类型注解，因为提供了<i32>类型的初始值
    //vec!是一个宏，会根据提供的值来创建一个新的vector
    ```

    修改vector

    ```rust
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    ```

    要加上mut注解才可以对vector进行修改，并且我们push进去的所有值都是i32类型的，所以不需要加\<i32>注解

    - 通常来说一个vector作用域结束时，其内容也都要被丢弃，但是如果内容存在引用呢？

      ```rust
      let v = vec![1, 2, 3, 4, 5];
      //如何读取vector中的元素：
      //1、索引
      let third: &i32 = &v[2];
      println!("The third element is {}", third);
      //2、get
      match v.get(2) {
          Some(third) => println!("The third element is {}", third),
          None => println!("There is no third element."),
      }
      
      //首先当vector中的元素存在引用时，不可以对vector进行修改
      let mut v = vec![1, 2, 3, 4, 5];
      let first = &v[0];
      v.push(6);//error
      println!("The first element is: {}", first);
      
      //遍历vector
      let v = vec![100, 32, 57];
      for i in &v {
          println!("{}", i);
      }
      //可变引用便利vector并修改值
      let mut v = vec![100, 32, 57];
      for i in &mut v {
          *i += 50;
      }
      ```

    - pop(): 移除并返回vector的最后一个元素

- **泛型**

  - 泛型类型：\<T>，可以用于函数、结构体

    ```rust
    //函数 找出一个[...]中最大的元素
    //但是会报错，因为编译器会认为不是所有的类型都可以直接用大于号 > 来进行比较
    fn largest<T>(list: &[T]) -> &T {
        let mut largest = list[0];
    
        for &item in list.iter() {
            if item > largest {
                largest = item;
            }
        }
    
        largest
    }
    
    fn main() {
        let number_list = vec![34, 50, 25, 100, 65];
    
        let result = largest(&number_list);
        println!("The largest number is {}", result);
    
        let char_list = vec!['y', 'm', 'a', 'q'];
    
        let result = largest(&char_list);
        println!("The largest char is {}", result);
    }
    
    //结构体
    struct Point<T> {
        x: T,
        y: T,
    }
    
    fn main() {
        let integer = Point { x: 5, y: 10 };
        let float = Point { x: 1.0, y: 4.0 };
    }
    ```

    在函数/结构体名称后面用尖括号声明泛型参数的名称

    但是这里要求x和y是同一类型的

    ```rust
    struct Point<T> {
        x: T,
        y: T,
    }
    
    fn main() {
        let wont_work = Point { x: 5, y: 4.0 };
    }//error 给x赋值5时，就等于告诉编译器这个泛型T是整型，所以再给y赋值为浮点型就会报错
    //正确写法：用两个泛型参数T和U
    struct Point<T, U> {
        x: T,
        y: U,
    }
    
    fn main() {
        let both_integer = Point { x: 5, y: 10 };
        let both_float = Point { x: 1.0, y: 4.0 };
        let integer_and_float = Point { x: 5, y: 4.0 };
    }
    ```

    定义中可以使用任意多的泛型参数

  - 枚举定义中的泛型

    标准库中的Option\<T>

    ```rust
    enum Option<T> {
        Some(T),
        None,
    }
    ```

    Option\<T> 是一个拥有泛型 `T` 的枚举，它有两个成员：`Some`，它存放了一个类型 `T` 的值，和不存在任何值的 `None`

  - 方法定义中的泛型

    对于带有泛型的结构体和枚举，有对应的泛型实例方法定义

    ```rust
    struct Point<T> {
        x: T,
        y: T,
    }
    
    impl<T> Point<T> {
        fn x(&self) -> &T {
            &self.x
        }
    }
    
    fn main() {
        let p = Point { x: 5, y: 10 };
    
        println!("p.x = {}", p.x());
    }
    ```

    注意必须在impl后面声明\<T>，这样Rust就知道Point后的尖括号中的T是泛型类型而不是某个具体类型

    - 也可以为某种特定类型实例实现方法，比如为Point\<f32>实现方法

      ```rust
      # struct Point<T> {
      #     x: T,
      #     y: T,
      # }
      #
      impl Point<f32> {
          fn distance_from_origin(&self) -> f32 {
              (self.x.powi(2) + self.y.powi(2)).sqrt()
          }
      }
      ```

      即只有Point\<f32>类型会有一个distance_from_origin的方法，而其他类型的Point\<T>没有此方法

    - 结构体定义中的泛型类型参数并不总是和结构体方法签名中使用的泛型是同一类型

      ```rust
      struct Point<T, U> {
          x: T,
          y: U,
      }
      
      impl<T, U> Point<T, U> {//impl后的<T,U>声明是用来表示后面的Point<T,U>是泛型类型
          fn mixup<V, W>(self, other: Point<V, W>) -> Point<T, W> {
              Point {
                  x: self.x,
                  y: other.y,
              }
          }
      }
      
      fn main() {
          let p1 = Point { x: 5, y: 10.4 };
          let p2 = Point { x: "Hello", y: 'c'};
      
          let p3 = p1.mixup(p2);
      
          println!("p3.x = {}, p3.y = {}", p3.x, p3.y);
      }
      ```

- **trait**：定义共享的行为

  - trait告诉编译器，某个特定类型拥有可能与其他类型共享的功能，可以使用trait bounds执行泛型是任何拥有特定行为的类型 （trait类似于其他语言中的借口）

    - 一个类型的行为由其可供调用的方法构成。如果可以对不同类型调用相同的方法，那么这些类型就共享相同的行为。trait定义就是一种将方法签名组合起来的方法，目的是定义一个实现某些目的所必需的行为集合

  - 用trait关键字来定义一个trait

    ```rust
    pub trait Summary {
        fn summarize(&self) -> String;
    }
    ```

    在花括号内声明描述实现这个trait的类型所需要的行为的方法签名 fn summarize(&self) -> String;

    注意，这里没有对该方法的具体实现，而是在签名后跟了个分号，因为需要**每一个实现这个trait的类型都提供一个自定义行为的这个方法体**，并且编译器也会保证每个实现该Summary trait的类型都有一个完全一致的summarize方法

    ```rust
    pub struct NewsArticle {
        pub headline: String,
        pub location: String,
        pub author: String,
        pub content: String,
    }
    
    impl Summary for NewsArticle {//为NewsArticle类实现Summary trait
        fn summarize(&self) -> String {
            format!("{}, by {} ({})", self.headline, self.author, self.location)
        }
    }
    ```

    实现之后也是和object.something()同样的格式调用该trait方法

  - 前面将trait设置为pub公有是为了可以让别的crate也可以使用该trait

    - 注意，只有当trait**或**要实现trait的类型位于crate的本地作用于时，才能为该类型实现该trait
      - 可以为上面这个结构体NewsArticle实现Display trait，因为NewsArticle就定义在当前crate内
      - 也可以为Vec实现Summary trait，因为Summary trait定义在当前crate内
      - 但是不可以在这里为Vec实现一个新的Display trait，因为这俩都定义在标准库中，没有定义在当前库。
    - 这保证了别人编写的代码不会破坏你的代码

  - 前面说trait中只需要声明方法签名，具体实现留给想要实现该trait方法的类型即可。但是也可以在trait定义中做**默认实现**，然后在为某个特定类型实现trait时，可以选择保留或重载方法的具体行为。

    ```rust
    pub trait Summary {
        fn summarize(&self) -> String {
            String::from("(Read more...)")
        }
    }
    ```

    如果想要NewsArticle实例使用这个默认实现，而不是定义一个自己的视线，那么可以通过impl Summary for NewsArticle{}指定一个空的impl块即可。

    ```rust
    pub trait Summary {
        fn summarize(&self) -> String {
            String::from("(Read more...)")
        }
    }
    pub struct NewsArticle {
        pub headline: String,
        pub location: String,
        pub author: String,
        pub content: String,
    }
        
    impl Summary for NewsArticle {}
    
    fn main(){
        let article = NewsArticle {
            headline: String::from("Penguins win the Stanley Cup Championship!"),
            location: String::from("Pittsburgh, PA, USA"),
            author: String::from("Iceburgh"),
            content: String::from("The Pittsburgh Penguins once again are the best
            hockey team in the NHL."),
        };
        println!("New article available! {}", article.summarize());
    }
    ```

  - 默认实现允许调用**相同trait中的其他方法**，哪怕这些方法没有默认实现（只需要实例调用者对应的类型实现了即可）

  - **trait作为参数**

    - 可以定义参数为impl某个trait类型的函数

      ```rust
      pub fn notify(item: impl Summary) {
          println!("Breaking news! {}", item.summarize());
      }
      ```

      item可以传入任意一个实现了Summary trait的类型参数，函数体中可以调用任意来自Summary trait的方法

    - **trait bound**：https://blog.csdn.net/readlnh/article/details/87276321

      上面的函数也可以写为如下形式：

      ```rust
      pub fn notify<T: Summary>(item: T) {
          println!("Breaking news! {}", item.summarize());
      }
      ```

      bound可以理解为，对泛型做一些范围限制，比如这里指定只能接收实现了Summary trait的类型

    - 但是什么时候应该写成trait bound的形式，什么时候又应该写成impl trait的形式呢？

      考虑这样一种情况：函数需要接收两个实现了Summary trait的类型

      ```rust
      pub fn notify(item1: impl Summary, item2: impl Summary) {
      ```

      这里的item1和item2允许是不同的类型，只要它们都实现了Summary trait

      那么如果想要限制item1和item2是同一类型，就要用trait bound的写法：

      ```rust
      pub fn notify<T: Summary>(item1: T, item2: T) {
      ```

      还可以通过+ 来制定多个trait

      ```rust
      pub fn notify(item: impl Summary + Display) {
      pub fn notify<T: Summary + Display>(item: T) {
      ```

      还可以用where关键字来简化代码，当多个泛型有多个trait bound时简化代码的可读性

      ```rust
      fn some_function<T: Display + Clone, U: Clone + Debug>(t: T, u: U) -> i32 { 
      // 变为=>
      fn some_function<T, U>(t: T, u: U) -> i32
          where T: Display + Clone,
                U: Clone + Debug
      {
      ```

  - **返回trait**：在返回值中使用impl trait语法，来返回某个实现了某个trait的类型

    ```rust
    fn returns_summarizable() -> impl Summary {
        Tweet {
            username: String::from("horse_ebooks"),
            content: String::from("of course, as you probably already know, people"),
            reply: false,
            retweet: false,
        }
    }
    ```

    注意，对于返回值是impl Summary的函数，不能讲其作为右值赋给一个**具体类型**的变量，因为这相当于一个模糊类型，不能赋给已有具体类型的变量，比如这里这个函数其实是返回了一个Tweet类型的实例，但是即使是一个Tweet类型的变量也不能接受该函数的返回值，即let tweet: Tweet = returns_summarizable();类似的语句是不允许的，当然去掉 “: Tweet” 的类型声明就可以了

  - 可以使用带有trait bound泛型参数的impl块，为实现了特定trait的类型实现方法

    ```rust
    use std::fmt::Display;
    
    struct Pair {
        x: T,
        y: T,
    }
    
    impl Pair {
        fn new(x: T, y: T) -> Self {
            Self {
                x,
                y,
            }
        }
    }
    
    impl<T: Display + PartialOrd> Pair {//对实现了Display和PartialOrd trait的类型实现方法
        fn cmp_display(&self) {
            if self.x >= self.y {
                println!("The largest member is x = {}", self.x);
            } else {
                println!("The largest member is y = {}", self.y);
            }
        }
    }
    ```

  - 也可以对实现了特定trait的类型实现其他trait

    ```rust
    impl<T: Display> ToString for T {
        // --snip--
    }
    ```

    上述是标准库中对所有实现了Display trait的类型也实现ToString trait