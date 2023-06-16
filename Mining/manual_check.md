[TOC]

#### 1、变量可变性改动

##### 方法参数声明中的可变性

1、cargo 81dfab4450 

```rust
-fn tap<F: FnOnce(&mut Self)>(mut self, callback: F) -> Self;                                                                   +fn tap<F: FnOnce(&mut Self)>(self, callback: F) -> Self;
```

参数声明中删除mut

Rust语言中的`Tap` trait是一个功能性的trait，它允许你在一个方法链中插入一个中间步骤以进行调试和打印。通常情况下，当你使用函数式编程风格时，你会需要构建一个数据处理管道。在这种情况下，使用`Tap` trait可以方便地将处理过程中的中间步骤输出到控制台上，以便于排除问题和了解管道的执行情况。

tap用于在数据处理的管道中能够查看对应的值（不可变）

tap_mut用于在数据处理的管道中能够修改对应的值（可变）

所以对于tap，其参数self应该是不可变的，tap_mut的self参数是可变的

> 有静态提示且无法编译，应该归为版本相关：在https://github.com/rust-lang/rust/issues/35203之后，编译器不再允许函数声明中出现pattern
>
> ```error: patterns aren't allowed in functions without bodies
> error: patterns aren't allowed in functions without bodies
> --> src/main.rs:2:34
> |
> 2 |     fn tap<F: FnOnce(&mut Self)>(mut self, callback: F) -> Self;
> |                                  ^^^^^^^^ help: remove `mut` from the parameter: `self`
> |
> = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
> = note: for more information, see issue #35203 <https://github.com/rust-lang/rust/issues/35203>
> = note: `#[deny(patterns_in_fns_without_body)]` on by default
> ```



2、rust 80f7db63b6    对vector，获取其可变切片，显然需要其本身是可变的

```rust
-pub fn as_mut_slice(&self) -> &mut [T] {                                                                                       +pub fn as_mut_slice(&mut self) -> &mut [T] {
```

参数声明中添加一个mut

This was intended to require `&mut self`, not `&self`, otherwise it's unsound!

> 有静态提示（clippy）但是能编译：在https://rust-lang.github.io/rust-clippy/master/index.html#mut_from_ref提到，这可能允许从一个变量的不可变引用生成多个可变引用，是unsound的
>
> 这个bug是从错误https://github.com/rust-lang/rust/issues/39465中发现的，修复后导致Rust版本从1.15.0升级至1.15.1
>
> ```error: mutable borrow from immutable input(s)
> error: mutable borrow from immutable input(s)
> --> src/main.rs:59:35
> |
> 59 |     pub fn as_mut_slice(&self) -> &mut [T] {
> |                                   ^^^^^^^^
> |
> note: immutable borrow here
> --> src/main.rs:59:25
> |
> 59 |     pub fn as_mut_slice(&self) -> &mut [T] {
> |                         ^^^^^
> = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#mut_from_ref
> = note: `#[deny(clippy::mut_from_ref)]` on by default
> ```



3、solana edf5bc242c

```rust
-pub fn serialize_data<T: serde::Serialize>(&mut self, state: &T) -> Result<(), bincode::Error> {                               +pub fn serialize_data<T: serde::Serialize>(&self, state: &T) -> Result<(), bincode::Error> {
    
    pub fn serialize_data<T: serde::Serialize>(&self, state: &T) -> Result<(), bincode::Error> {
        if bincode::serialized_size(state)? > self.data_len() as u64 {
            return Err(Box::new(bincode::ErrorKind::SizeLimit));
        }
        bincode::serialize_into(&mut self.data.borrow_mut()[..], state)
    }
```

参数声明中删除mut

需要用到self.data.borrow_mut()，其中self.data是RefCell的形式，所以self本身并不需要是可变的

而且如果设置为&mut self的话，无法对不可变的实例调用该方法，因为需要&mut，只能对可变实例调用

4、rust c3d6ee9e7b

```rust
543     pub fn find_breakable_scope(&mut self,                    543     pub fn find_breakable_scope(&self,
544                            span: Span,                        544                            span: Span,
545                            label: region::Scope)              545                            label: region::Scope)
546                            -> &mut BreakableScope<'tcx> {     546                            -> &BreakableScope<'tcx>{
547         // find the loop-scope with the correct id            547         // find the loop-scope with the correct id
548         self.breakable_scopes.iter_mut()                      548        self.breakable_scopes.iter()  
    
    
    self.breakable_scopes.iter()
            .rev()
            .filter(|breakable_scope| breakable_scope.region_scope == label)
            .next()
            .unwrap_or_else(|| span_bug!(span, "no enclosing breakable scope found"))
    }
```

参数声明中删除mut，下文中不需要修改参数的值

**没有静态提示**





##### 获取变量的引用方法的改动

1、rust 46a683111d 

```rust
-unsafe { f(waker_ptr.as_mut()) }                                                                                               +unsafe { f(waker_ptr.as_ref()) }
```

将pointer::as_mut()获取可变引用改为使用pointer::as_ref()获取不可变引用，因为前面已经有一个不可变引用了，违背了stacked borrow原则



##### 删去变量unused mut

1、cargo 90d0b120be

```rust
-let mut cx = Context::new(config, &bcx)?;                                                                                       +let cx = Context::new(config, &bcx)?;

let mut cx = Context::new(config, &bcx)?;
cx.compile(&units, export_dir.clone(), &exec)?


-let mut client = BufReader::new(client);
+let client = BufReader::new(client);

let client = BufReader::new(client);
match serde_json::from_reader(client) {
    Ok(message) => on_message(message),
    Err(e) => warn!("invalid diagnostics message: {}", e),
}
```

2、servo 08987c6f5a

```rust
-let mut flags = self.base.flags;                                                                                               +let flags = self.base.flags;
```

 

#### 2、指针与类型改动

##### 删除解引用符*



**1、gfx a999cb37a7**

```rust
-let mut transition = *bar.u.Transition_mut();                                                                                   +let mut transition = bar.u.Transition_mut();
```

运算优先级：先取field再解引用

Transition_mut()是一个unsafe函数，将一个类型T1的&mut转为另一个类型T2的&mut，对这个&mut做解引用以进行赋值，若T2实现了Copy trait，那么就会按位拷贝一份；若没有实现Copy trait，则会报错。这边是实现了Copy trait，但是本意并不是用新的一份，而是旧的那份



2、solana 2f5102587c

```rust
-account.owner(),                                                                                                   +*account.owner(),

let mut rewarded_accounts = modified_accounts
    .iter()
    .map(|(pubkey, account)| {
        (
            pubkey,
            account,
            base_bank
                .get_account(&pubkey)
                .map(|a| a.lamports)
                .unwrap_or_default(),
        )
    })
    .collect::<Vec<_>>();
rewarded_accounts.sort_unstable_by_key(
    |(pubkey, account, base_lamports)| {
        (
            account.owner(),
            *base_lamports,
            account.lamports - base_lamports,
            *pubkey,
        )
    },
);

impl ReadableAccount for Account {
    fn owner(&self) -> &Pubkey {
        &self.owner
    }
}

```

Intermittent lifetime issue with compiler likely introduced with owner() refactoring.

https://zhuanlan.zhihu.com/p/447710476?utm_id=0



3、solana f2ee01ace3 （去掉了一个引用&，等于加上了一层解引用）

```rust
-&entries,                                                                                                                       +entries,
```

这是一个函数参数中的内容，形参就是不带引用的



4、wezterm 81d5a92b66（先解引用后引用）

~~~rust
-if existing.data() == data {                                                                                                   +if existing.data() == &*data {
    
Build fix for `no implementation for `&[u8] == std::vec::Vec<u8>`

Full error
```
error[E0277]: can't compare `&[u8]` with `std::vec::Vec<u8>`
   --> wezterm-gui/src/gui/termwindow.rs:817:40
    |
817 |                     if existing.data() == data {
    |                                        ^^ no implementation for `&[u8] == std::vec::Vec<u8>`
    |
    = help: the trait `std::cmp::PartialEq<std::vec::Vec<u8>>` is not implemented for `&[u8]`

error: aborting due to previous error
```
~~~

右边的data是std::vec::Vec\<u8>，左边是&[u8]，先对data解引用获得[u8]，再对其引用获得&[u8]



##### 类型别名/声明的改动

1、rust bdb53e55b0

```rust
-pub type pthread_t = usize;                                                                                                     +pub type pthread_t = u32;

libc::pthread_kill(self.thread.as_pthread_t(), libc::SIGUSR1);
```

Fix the Solaris pthread_t raw type in std to match what's in libc

有两个pthread_t（libc和std库），这里改的是std里的。在libc::pthread_kill()中第一个参数是libc::pthread_t，而as_pthread_t()方法是std的，返回一个RawPthread类型，RawPthread是 std::os::linux::raw::pthread_t类型，那么就要看libc的pthread_t和std的pthread_t都分别是什么类型的重名。libc::pthread_t是c_uint/u32，而std中的pthread_t原本是usize，所以需要将std中的pthread_t类型重名改为u32



2、wezterm 9a6cee2b59

```rust
-message: *const i8,                                                                                                             +message: *const c_char,

let message = CStr::from_ptr(message);
```

fix the build on non-x86 architectures

where `c_char` is an `u8` instead of an `i8`

Rust中的c_char相当于C语言中的char，而Rust的char不同于C中的char，Rust中的char是unicode scalar value，而C的char本质上就是一个整型数。所以Rust中的c_char是整型数的别名，在不同的架构中c_char相当于u8或i8。

在有些体系架构下，char是unsigned的，比如aarch64，所以设置为i8会出现问题。

后面的CStr::from_ptr接受的也是\*const c_char而不是\*const i8

**portability**https://github.com/rust-lang/rust/issues/79089



3、nushell 2fe14a7a5a

```rust
-if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<usize>().is_err() {                                             +if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<u64>().is_err() {
```

fix timestamp parsing on 32-bit platforms

https://github.com/nushell/nushell/issues/5191

32位设备上usize是u32，但是时间戳输入的会爆u32，要用64位存时间戳



##### 类型转换的改动

1、alacritty 02953c2812

```rust
-libc::ioctl(fd, TIOCSCTTY as u64, 0)                                                                                           +libc::ioctl(fd, TIOCSCTTY as _, 0)
```

Fix `ioctl` call failing on 32 bit architecture

TIOCSCTTY在64位设备上跑是u64 as u64，但在macos上是u32 as u64

Type inference regression推断类型回归，给编译器自己推断类型



2、alacritty 92ea355eee

```rust
-use libc::c_uint;                                                                                                               +use libc::{c_long, c_uint};

-fn to_fixedpoint_16_6(f: f64) -> i64 {                                                                                         +fn to_fixedpoint_16_6(f: f64) -> c_long {

-(f * 65536.0) as i64                                                                                                           +(f * 65536.0) as c_long
    let xx = to_fixedpoint_16_6(matrix.xx);
    let xy = to_fixedpoint_16_6(matrix.xy);
    let yx = to_fixedpoint_16_6(matrix.yx);
    let yy = to_fixedpoint_16_6(matrix.yy);
    Matrix { xx, xy, yx, yy }
```

Fix compilation on 32bit targets

用到这个方法的是freetype-rs::Matrix，其中的元素类型是libc::c_long，而不是i64，c_long在不同的架构上可能代表i32也可能代表i64



3、rust 3f0462acbb

```rust
-let mut nonblocking = nonblocking as libc::c_ulong;                                                                             +let mut nonblocking = nonblocking as libc::c_int;

cvt(unsafe { libc::ioctl(*self.as_inner(), libc::FIONBIO, &mut nonblocking) }).map(|_| ())
```

一个函数需要一个指向int的指针（且该值只可能为0或1），这里的nonblocking原本作为c_ulong的指针传入，在all 32-bit platforms and on all litte-endian platforms都没事，但会break on big-endian 64-bit platforms.



4、wasmer 6cc41f82c8

```rust
-let ret = unsafe { lseek(fd, offset, whence) };                                                                                 +let ret = unsafe { lseek(fd, offset, whence) as i64 };
...
let result_ptr = result_ptr_value.deref(ctx.memory(0)).unwrap();
result_ptr.set(ret);

libc::lseek源码：
pub fn lseek(fd: ::c_int, offset: off_t, whence: ::c_int) -> off_t;
```

Fixed lseek error in Windows 

todo

result_ptr是&Cell\<i64\>，调用.set()方法的参数要是i64的



#### 3、内存安全相关

**1、rust 928efca151 通过修改 `get_mut` 为 `as_mut_ptr` 来避免未定义行为**

获取可变引用改为获取裸指针

```rust
let mut out = MaybeUninit::uninitialized();
...
-"{rcx}"(out.get_mut())                                                                                                         +"{rcx}"(out.as_mut_ptr())

let mut report = MaybeUninit::uninitialized();
...
-"{rdx}"(report.get_mut())                                                                                                       +"{rdx}"(report.as_mut_ptr())
```

对于未初始化的对象，获取其引用可能产生未定义行为，而获取其裸指针不会



**2、rust 763392cb8c 添加手动释放内存解决原生指针内存泄漏问题**

```rust
+drop(Box::from_raw(p));
```



**3、rust 8341f6451b**

```rust
-unsafe { *(0 as *mut isize) = 0; }                                                                                             +unsafe { *(1 as *mut isize) = 0; }
```

Fix run-pass/signal-exit-status to not trigger UB by writing to NULL.

这段代码就是为了引发一个段错误，但是向NULL指针写数据会触发未定义行为



#### 4、版本相关

##### stable和nightly版本中的方法不兼容

1、parity-ethereum ec9c6e9783

```rust
-info!(target: "network", "Public node URL: {}", Colour::White.bold().paint(public_url.as_ref()));
+info!(target: "network", "Public node URL: {}", Colour::White.bold().paint(AsRef::<str>::as_ref(public_url)));
```

On nightly rust passing `public_url` works but that breaks on stable. This works for both.



##### trait对象添加dyn关键字

1、deno 056c146175

```rust
-type Target = Box<AnyError>;                                                                                               +type Target = Box<dyn AnyError>;
```

Fix expected dyn before AnyError trait (#2663)



2、rust 50057ee3a3

```rust
-let mut y = &mut x as &mut Any;                                                                                                 +let mut y = &mut x as &mut dyn Any;
```

bench: libcore: fix build failure of any.rs benchmark (use "dyn Any")



3、wasmer 6372e0947c

```rust
-static LAST_ERROR: RefCell<Option<Box<Error>>> = RefCell::new(None);                                                         +static LAST_ERROR: RefCell<Option<Box<dyn Error>>> = RefCell::new(None);

-pub(crate) fn take_last_error() -> Option<Box<Error>> {                                                                         +pub(crate) fn take_last_error() -> Option<Box<dyn Error>> {
```

Fix more bare dyn traits in runtime-c-api



4、solana 3f0480d060

```rust
-let mut executable = Executable::from_elf(&data, None, config).unwrap();                                                       +let mut executable = <dyn Executable::<BpfError, ThisInstructionMeter>>::from_elf(&data, None, config).unwrap();
```

Fix deprecated trait object without an explicit dyn warning (#17231)





#### 5、无锁编程中的Memory ordering

**1、solana 04d23a1597**

```rust
-self.current_len.store(0, Ordering::Relaxed);                                                                                   +self.current_len.store(0, Ordering::Release);

-self.current_len.load(Ordering::Relaxed)                                                                                       +self.current_len.load(Ordering::Acquire)

-let aligned_current_len = u64_align!(self.current_len.load(Ordering::Relaxed));                                                 +let aligned_current_len = u64_align!(self.current_len.load(Ordering::Acquire));

-self.current_len.store(*offset, Ordering::Relaxed);                                                                           +self.current_len.store(*offset, Ordering::Release);
```



**2、solana 38cd29810f**

```rust
-self.ref_count.load(Ordering::Relaxed)                                                                                         +self.ref_count.load(Ordering::Acquire)

-self.ref_count.fetch_add(1, Ordering::Relaxed);                                                                                 + self.ref_count.fetch_add(1, Ordering::Release);

-self.ref_count.fetch_sub(1, Ordering::Relaxed);                                                                                 + self.ref_count.fetch_sub(1, Ordering::Release);
```



**3、solana ddd0ed0af1**

```rust
-self.last_age_flushed.store(age, Ordering::Relaxed);                                                                           +self.last_age_flushed.store(age, Ordering::Release);

-self.last_age_flushed.load(Ordering::Relaxed)                                                                                   +self.last_age_flushed.load(Ordering::Acquire)

```



**4、rust af047d9c10**

```rust
+cur = this.inner().weak.load(Relaxed);
```

Fix infinite loop in Arc::downgrade



聚类指标

node type频率

**具体有哪些pattern**、pattern中和Rust特性相关的、lessons learned

