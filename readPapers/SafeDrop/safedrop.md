interior unsafe --> 

current Rust compiler has done little regarding the memory-safety risks of unsafe code but simply assumes developers should be responsible for employing them



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
  
  - Rust compiler strips ‘unsafe’ markers before lowering the code to MIR, thus the borrow checker is valid for both **safe** Rust and **unsafe** Rust. 
  
    However, this system exists several drawbacks. The sound lifetime inference and borrow checker ensure the safety of **references only**. Any unsafe code interacted with raw pointers can breach the safety promise and may lead to memory reclaim. 
  
  - Rust enforces RAII and releases unused resources automatically. In practice, this mechanism may falsely drop some buffers and is prone to memory-safety issues. 
  
    - according to the buggy Drop() terminator, such problems are divided into 2 categories: invalid drop in normal execution path and invalid drop in exception handling path
  
      **invalid drop in normal execution**: Drop() terminator locates in a normal execution path, and the parameter of Drop() is not safe to launch. 
  
      ```rust
      fn genvec() -> Vec<u8>{
        let mut s = String::from("a␣tmp␣string");
        let ptr = s.as_mut_ptr();
        let v;
        unsafe{
          v = Vec::from_raw_parts(
            ptr, s.len(), s.len());
        }
      }
      // mem::forget(s); // do not drop s
      // otherwise, s is dropped before return return v;
      }
      fn main() {
      	let v = genvec();
      	// use v -> use after free
      	// drop v before return -> double free
      }
      ```
  
      the string **s** and the vector **v** are sharing the same memory space. 
  
      After the automatic deallocation of the string, the vector will contain a dangling pointer pointing to the released buffer —— potential use-after-free problem. 
  
      As long as the control-flow goes out of its drop scope, it will be dropped automatically —— double free problem.
  
      - correspond MIR: 
  
        _1 in bb0 creates a new string and the returned vector _0 in bb5 is created based on _1 with an alias propagation chain 1->5->4->3->2->8->0.
  
        So _0 contains an alias pointer of _1 and drop(\_1) makes _0 a dangling pointer and drop(\_0) afterwards will lead to double-free problem
  
      - Why cannot borrow checker add the support for raw pointers to detect such issues?
  
        For simplicity, Rust assumes that each parameter should either transfer its ownership to the callee for the drop-trait variable or duplicate a deep copy for the copy-trait variable, and the return value would no longer share the ownership with the alive variables remained.
  
        Note that from_raw_parts() is an unsafe function —— maybe lead to memory reclaim. 
  
        And the current Rust compiler does not add the alias checking support for raw pointers
  
      **Invalid drop of exception handling**
  
      - Double-free and drop uninitialized memory might arise during the unwinding process after the program panics
      - if develpers add mem::forget() to the code in Figure 2a which prevents drop(_1), they may also add more statements between creating v and calling mem::forget(). As long as the program panics during the execution of these statements, Rust should deallocate resources during stack unwinding by continuously calling Drop(). Again, double-free problem would arise.
      - Likewise, dropping uinitialized memory is also possible during exception handling.
  
- problem definition
  
  - **Dropping buffers in use**
    - falsely deallocates some buffers that will be accessed later, including use-after-free and double-free.
  
  - **Dropping invalid pointers**
    - dangling pointers: double free
    - point to uninitialized memory: invalid memory access
  
- typical patterns
  
  - use-after-free
  - double-free
  - Invalid memory access (use or drop directly) 
  
- Research Challenge
  
  - Dynamic analysis
    - It is not easy to set up certain conditions to trigger buggy scenarios.
  
    - e.g., fuzzing can hardly generate test cases to cover all the panic unwinding paths.
  
    - There are also difficult memory modeling issues for path constraint extraction.
  
  - Static analysis
    - alias relationship involves
      - move
  
      - mutable borrow
  
      - immutable borrow
  
      - dereference
  
    - Only **Drop variable** will be automatically deallocated (traits of compound types can be derived from its subtypes) —— need to infer the traits of each type.
  
    - NP-hard alias analysis —— sacrifice precision —— false positive
  
- Approach detecting invalid memory deallocation problems

  - **path-sensitive data-flow analysis**

    - input: MIR of functions 

    - output: warnings of potential invalid memory deallocation issues along with corresponding buggy code snippets. 

    - key steps

      - path extraction
      - alias analysis
      - invalid drop detection

    - **Path extraction**

      - Meet-over-paths: traverse CFG of a function and enumerate all **valuable** paths 

        - What paths are "valuable"?
          1. a unique set of code blocks with an entrance and an exit
          2. should not be the subset of another valuable path (select the bigger set) —— avoid traversing cycled blocks repeatedly, just consider the maximum set of blocks

      - a modified tarjan algorithm to remove redundant paths

        - decompose strongly connected components (SCC) of the graph and removes the cycle succinctly --> a DAG

        - generate a spanning tree

        - enumerate all the valuable paths (ideally)

        - above uses the traditional Tarjan Algorithm, but less accurate for some particular statements

          Exception example (enumertation) : need to prune unreachable paths and construct independent paths for different variants 

    - **Alias Analysis** (inter-procedure)

      - perform for each path and establish the alias sets for each program point 
      - Basic rules:
        - not all alias are critical, i.e. we only focus on Drop-trait variables (skip copy-trait [stack-only] variables as well as composite types whose components are all filtered out recursively and are copy-trait)
        - summerize 5 kinds of statements that contribute to alias relationships for Lvalue and Rvalue (use the structure of union-find disjoint set).

      - Inter-procedural alias annalysis
        - For function calls, i.e. involving parameters and return value

    - **Invalid drop detection**

      - based on alias sets obtained in the alias analysis; once detect a memory deallocation bug, record it with related code and then merge the result at the end of this path
      - maintain a taint set to record the deallocated buffers as well as returned dangling pointers; add the dropping variable into the taint set and marks it as the taint source when finding Drop() in the terminator, and the taint source propagates in the alias set and pollutes other aliases (add uninitialized variables as well when declaring, and remove them once they are initialized)
      - 4 rules
        - use-after-free: the taint set contains the alias of variable in a statement
        - Double-free: the taint set contains the alias of variable in the drop() terminator
        - Invalid memory access: the taint set contains an uninitialized variable in a statement or drop()
        - Dangling-pointer: the taint set contains the returned pointer

- Evaluation

  - implementation
    - Integrated into the Rust compiler v1.52 and can be used by the command line tools such as rustc and cargo

  - corresponding CVEs covered —— 2 use-after-free issues, 3 double-free, 4 invalid memory access from 8 different Rust crates
  - Evaluated 24 re al-world Rust crates from Github











## Corpus

- isahc
- Open-ssl
- linea
- ordnung
- crossbeam https://github.com/crossbeam-rs/crossbeam
- generator
- linkedhashmap
- Smallvec

# related work

### Formal verification

- **Leveraging rust types for modular specification and verification.** [Proc. ACM Program. Lang. 3(OOPSLA)](https://dblp.org/db/journals/pacmpl/pacmpl3.html#Astrauskas0PS19): 147:1-147:30 (2019)
  - re: exclusive capability —— mutable memory location
- RustBelt Meets Relaxed Memory. 4, POPL, Article 34 (2019), 29 pages.
- RustBelt: Securing the Foundations of the Rust Programming Language. 2, POPL, Article 66 (Dec. 2017), 34 pages.
- Crust: A Bounded Veriier for Rust (N). In 2015 30th IEEE/ACM International Conference on Automated Software Engineering (ASE ’15). 75-80.



## Unsafe code

- How do programmers use unsafe rust? Proceedings of the ACM on Programming Languages 4, OOPSLA (2020),27.
  - analyse a large corpus of Rust projects to assess the validity of the Rust hypothesis and to classify the purpose of unsafe code
    - use unsafe code sparingly, 
    - make it easy to review, 
    - and hide it behind a safe abstraction such that client code can be written in safe Rust
  - classify several motivations for using unsafe code
  - manually review
- **corpus**
  - https://github.com/nrc/r4cppp Accessed May 11, 2020.
  - Fuchsia Team. 2020. Fuchsia Documentation - Unsafe Code in Rust. https://fuchsia.googlesource.com/fuchsia/+/master/docs/development/languages/rust/unsafe.md Accessed May 11, 2020. 
  - Jon Gjengset. 2020. Demystifying unsafe code (Talk at Rust NYC). https://youtu.be/QAz-maaH0KM Accessed on March 19, 2020.
  - Redox developers. 2019. Snippet from Redox OS Repository. https://github.com/redox-os/relibc/blob/2cbc78f238b3eda426171def100f44707cfe8ae3/src/platform/pte.rs#L337-L345 Accessed May 11, 2020.
  - Rust Team. 2019a. Mission Statement of the Secure Code Working Group. https://github.com/rust-secure-code/wg Accessed May 11, 2020.
  - Rust Team. 2020a. File: check_unsafety.rs. https://github.com/rust-lang/rust/blob/27ae2f0d60d9201133e1f9ec7a04c05c8e55e665/src/librustc_mir/transform/check_unsafety.rs Accessed May 11, 2020
  - Qrates Team. 2020. Qrates artefact. https://doi.org/10.5281/zenodo.4085004 Source code and dataset: https://github.com/rust-corpus/qrates. 
  - The Libra Association. 2020. Core Repository of the Libra Project. https://github.com/libra/libra/blob/8d9bba00629e602051e40bab2b80e7ed89f40c0b/storage/storage-client/src/state_view.rs#L96-L97 Accessed May 11, 2020. 



## Deal with unsafe code

- Fuzzing the Rust Type checker Using CLP. In Proceedings of the 30th IEEE/ACM International Conference on Automated Software Engineering (Lincoln, Nebraska) (ASE ’15). IEEE Press, 482-493.
  -  target the typechecker implementation for testing
- Sandcrust: Automatic Sandboxing of Unsafe Components in Rust. In Proceedings of the 9th Workshop on Programming Languages and Operating Systems (Shanghai, China) (PLOS’17).Association for Computing Machinery, New York, NY, USA, 51-57.
  - wrapping the C library’s API into remote procedure calls (RPC) to a library instance running in a sandboxed process (restful?)
- Securing Unsafe Rust Programs with XRust. In Proceedings of the ACM/IEEE 42nd International Conference on Software Engineering (Seoul, South Korea) (ICSE ’20). Association for Computing Machinery, New York, NY, USA, 234-245.
  - using instrumentation-based memory isolation —— unsafe and safe data are kept separatedly to avoid cross-region data flow
  - only less than 1% rust codes contain unsafe code



## Tools for Rust

-   **static analysis** tools: 

  - **Rudra**: Finding Memory Safety Bugs in Rust at the Ecosystem Scale. In Proceedings of the ACM SIGOPS 28th Symposium on Operating Systems Principles (SOSP ’21). 84-99.

    - Unsafe rust 通常有两种方法：
      - 内部 Unsafe API 直接暴露给 API 用户，但是使用 unsafe 关键字来声明该 API 是不安全的，也需要添加安全边界的注释。
      - 对 API 进行安全封装（安全抽象），即在内部使用断言来保证在越过安全边界时可以Panic，从而避免 UB 的产生。
      - 第二种方法，即将 Unsafe 因素隐藏在安全 API 之下的安全抽象，已经成为 Rust 社区的一种约定俗成。
    - Safe 和 Unsafe 的分离，可以让我们区分出谁为安全漏洞负责。Safe Rust 意味着，无论如何都不可能导致未定义行为。换句话说，Safe API 的职责是，确保任何有效的输入不会破坏内部封装的 Unsafe 代码的行为预期。

    这与C或C++形成了鲜明的对比，在C或C++中，用户的责任是正确遵守 API 的预期用法。

    - 三种错误模式：
      1. Panic Safety （恐慌安全）： 由恐慌导致的内存安全 Bug。
      2. Higher-order Safety Invariant（高阶安全不变性 ）：由高阶类型没有给定安全保证而引发的 Bug。
      3. Propagating Send/Sync in Generic Types（泛型中`Send/Sync`传播）：由泛型内部类型不正确的手工`Send/Sync`实现引起泛型 `Send/Sync` 约束不正确而引发的 Bug。

  - **MirChecker**: Detecting Bugs in Rust Programs via Static Analysis. In Proceedings of the 2021 ACM SIGSAC Conference on Computer and Communications Security (CCS ’21). 2183-2196. Miri. 2019. An interpreter for Rust’s mid-level intermediate representation. https://github.com/rust-lang/miri

    - runtime panic bugs (buffer overflow, integer overflow）同上

- **clippy**

  | 分组                | 描述                                                         | 默认级别  |
  | ------------------- | ------------------------------------------------------------ | --------- |
  | clippy::all         | 默认的所有lint (correctness, suspicious, style, complexity, perf) | warn/deny |
  | clippy::correctness | 完全错误或无用的代码                                         | deny      |
  | clippy::style       | 应该以更习惯的方式编写的代码                                 | warn      |
  | clippy::suspicious  | 很可能是错误或无用的代码                                     | warn      |
  | clippy::complexity  | 以复杂的方式完成简单工作的代码                               | warn      |
  | clippy::perf        | 可以被写得运行更快的代码                                     | warn      |
  | clippy::pedantic    | 相当严格的lint，但偶尔可能有误报                             | allow     |
  | clippy::nursery     | 仍在开发中的新lint                                           | allow     |
  | clippy::cargo       | 适用于cargo的lint                                            | allow     |

  编译器前端Rustc，Clippy基于Rustc提供的插件机制，将Clippy中的lints注册到Rustc的lint集合中，Rustc有专门的编译过程来执行这些lint检查。

  - **MIRAI** (linter) can be used to look for security bugs via taint analysis (information leaks, code injection bugs, etc.) and constant time analysis (information leaks via side channels). Unintentional (or ill-considered) panics can also become security problems (denial of service, undefined behavior).

- **dynamic analysis tool:**

  - **Miri**. 2019. An interpreter for Rust’s mid-level intermediate representation. https://github.com/rust-lang/miri

    - An experimental interpreter for [Rust](https://www.rust-lang.org/)'s [mid-level intermediate representation](https://github.com/rust-lang/rfcs/blob/master/text/1211-mir.md) (MIR). It can run binaries and test suites of cargo projects and detect certain classes of [undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html), for example:

      - Out-of-bounds memory accesses and use-after-free
      - Invalid use of uninitialized data
      - Violation of intrinsic preconditions (an [`unreachable_unchecked`](https://doc.rust-lang.org/stable/std/hint/fn.unreachable_unchecked.html) being reached, calling [`copy_nonoverlapping`](https://doc.rust-lang.org/stable/std/ptr/fn.copy_nonoverlapping.html) with overlapping ranges, ...)
      - Not sufficiently aligned memory accesses and references
      - Violation of *some* basic type invariants (a `bool` that is not 0 or 1, for example, or an invalid enum discriminant)
      - **Experimental**: Violations of the [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md) rules governing aliasing for reference types
      - **Experimental**: Data races

      On top of that, Miri will also tell you about memory leaks: when there is memory still allocated at the end of the execution, and that memory is not reachable from a global `static`, Miri will raise an error.

