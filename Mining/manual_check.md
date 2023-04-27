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

2、rust 80f7db63b6    对vector，获取其可变切片，显然需要其本身是可变的

```rust
-pub fn as_mut_slice(&self) -> &mut [T] {                                                                                       +pub fn as_mut_slice(&mut self) -> &mut [T] {
```

参数声明中添加一个mut

This was intended to require `&mut self`, not `&self`, otherwise it's unsound!

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

 

#### 2、类型改动

##### 删除解引用符*

1、gfx a999cb37a7

```rust
-let mut transition = *bar.u.Transition_mut();                                                                                   +let mut transition = bar.u.Transition_mut();
```

运算优先级：先取field再解引用



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



3、solana f2ee01ace3 （去掉了一个引用&，等于加上了一层解引用）

```rust
-&entries,                                                                                                                       +entries,
```



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



##### 类型定义的改动

1、rust bdb53e55b0

```rust
-pub type pthread_t = usize;                                                                                                     +pub type pthread_t = u32;
```

Fix the Solaris pthread_t raw type in std to match what's in libc



2、wezterm 9a6cee2b59

```rust
-message: *const i8,                                                                                                             +message: *const c_char,
```

fix the build on non-x86 architectures

where `c_char` is an `u8` instead of an `i8`



3、nushell 2fe14a7a5a

```rust
-if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<usize>().is_err() {                                             +if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<u64>().is_err() {
```

fix timestamp parsing on 32-bit platforms

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
```

Fix compilation on 32bit targets



3、rust 3f0462acbb

```rust
-let mut nonblocking = nonblocking as libc::c_ulong;                                                                             +let mut nonblocking = nonblocking as libc::c_int;
```

一个函数需要一个int型的指针，这里的nonblocking原本作为c_ulong的指针传入，在all 32-bit platforms and on all litte-endian platforms都没事，但会break on big-endian 64-bit platforms.



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

后面ret要作为参数传给别的函数，所以应该需要是直接转为i64

#### 3、内存安全相关

1、rust 928efca151 通过修改 `get_mut` 为 `as_mut_ptr` 来避免未定义行为

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



2、rust 763392cb8c 添加手动释放内存解决原生指针内存泄漏问题

```rust
+drop(Box::from_raw(p));
```



3、rust 8341f6451b

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

1、solana 04d23a1597

```rust
-self.current_len.store(0, Ordering::Relaxed);                                                                                   +self.current_len.store(0, Ordering::Release);

-self.current_len.load(Ordering::Relaxed)                                                                                       +self.current_len.load(Ordering::Acquire)

-let aligned_current_len = u64_align!(self.current_len.load(Ordering::Relaxed));                                                 +let aligned_current_len = u64_align!(self.current_len.load(Ordering::Acquire));

-self.current_len.store(*offset, Ordering::Relaxed);                                                                           +self.current_len.store(*offset, Ordering::Release);
```



2、solana 38cd29810f

```rust
-self.ref_count.load(Ordering::Relaxed)                                                                                         +self.ref_count.load(Ordering::Acquire)

-self.ref_count.fetch_add(1, Ordering::Relaxed);                                                                                 + self.ref_count.fetch_add(1, Ordering::Release);

-self.ref_count.fetch_sub(1, Ordering::Relaxed);                                                                                 + self.ref_count.fetch_sub(1, Ordering::Release);
```



3、solana ddd0ed0af1

```rust
-self.last_age_flushed.store(age, Ordering::Relaxed);                                                                           +self.last_age_flushed.store(age, Ordering::Release);

-self.last_age_flushed.load(Ordering::Relaxed)                                                                                   +self.last_age_flushed.load(Ordering::Acquire)

```



4、rust af047d9c10

```rust
+cur = this.inner().weak.load(Relaxed);
```

Fix infinite loop in Arc::downgrade
