fn foo(x: &'a str) { }

struct Foo {
    // error, use of undeclared lifetime name `'a`
    x: &'a str,
}
```

These can be fixed by declaring lifetime parameters:

```
fn foo<'a>(x: &'a str) {}

struct Foo<'a> {
    x: &'a str,
}
```
"##,

E0262: r##"
Declaring certain lifetime names in parameters is disallowed. For example,
because the `'static` lifetime is a special built-in lifetime name denoting
the lifetime of the entire program, this is an error:

```compile_fail
// error, invalid lifetime parameter name `'static`
fn foo<'static>(x: &'static str) { }
```
"##,

E0263: r##"
A lifetime name cannot be declared more than once in the same scope. For
example:

```compile_fail
// error, lifetime name `'a` declared twice in the same scope
fn foo<'a, 'b, 'a>(x: &'a str, y: &'b str) { }
```
"##,

E0264: r##"
An unknown external lang item was used. Erroneous code example:

```compile_fail
#![feature(lang_items)]

extern "C" {
    #[lang = "cake"] // error: unknown external lang item: `cake`
    fn cake();
}
```

A list of available external lang items is available in
`src/librustc/middle/weak_lang_items.rs`. Example:

```
#![feature(lang_items)]

extern "C" {
    #[lang = "panic_fmt"] // ok!
    fn cake();
}
```
"##,

E0269: r##"
Functions must eventually return a value of their return type. For example, in
the following function:

```compile_fail
fn foo(x: u8) -> u8 {
    if x > 0 {
        x // alternatively, `return x`
    }
    // nothing here
}
```

If the condition is true, the value `x` is returned, but if the condition is
false, control exits the `if` block and reaches a place where nothing is being
returned. All possible control paths must eventually return a `u8`, which is not
happening here.

An easy fix for this in a complicated function is to specify a default return
value, if possible:

```ignore
fn foo(x: u8) -> u8 {
    if x > 0 {
        x // alternatively, `return x`
    }
    // lots of other if branches
    0 // return 0 if all else fails
}
```

It is advisable to find out what the unhandled cases are and check for them,
returning an appropriate value or panicking if necessary.
"##,

E0270: r##"
Rust lets you define functions which are known to never return, i.e. are
'diverging', by marking its return type as `!`.

For example, the following functions never return:

```no_run
fn foo() -> ! {
    loop {}
}

fn bar() -> ! {
    foo() // foo() is diverging, so this will diverge too
}

fn baz() -> ! {
    panic!(); // this macro internally expands to a call to a diverging function
}
```

Such functions can be used in a place where a value is expected without
returning a value of that type, for instance:

```no_run
fn foo() -> ! {
    loop {}
}

let x = 3;

let y = match x {
    1 => 1,
    2 => 4,
    _ => foo() // diverging function called here
};

println!("{}", y)
```

If the third arm of the match block is reached, since `foo()` doesn't ever
return control to the match block, it is fine to use it in a place where an
integer was expected. The `match` block will never finish executing, and any
point where `y` (like the print statement) is needed will not be reached.

However, if we had a diverging function that actually does finish execution:

```ignore
fn foo() -> ! {
    loop {break;}
}
```

Then we would have an unknown value for `y` in the following code:

```no_run
fn foo() -> ! {
    loop {}
}

let x = 3;

let y = match x {
    1 => 1,
    2 => 4,
    _ => foo()
};

println!("{}", y);
```

In the previous example, the print statement was never reached when the
wildcard match arm was hit, so we were okay with `foo()` not returning an
integer that we could set to `y`. But in this example, `foo()` actually does
return control, so the print statement will be executed with an uninitialized
value.

Obviously we cannot have functions which are allowed to be used in such
positions and yet can return control. So, if you are defining a function that
returns `!`, make sure that there is no way for it to actually finish
executing.
"##,

E0271: r##"
This is because of a type mismatch between the associated type of some
trait (e.g. `T::Bar`, where `T` implements `trait Quux { type Bar; }`)
and another type `U` that is required to be equal to `T::Bar`, but is not.
Examples follow.

Here is a basic example:

```compile_fail
trait Trait { type AssociatedType; }

fn foo<T>(t: T) where T: Trait<AssociatedType=u32> {
    println!("in foo");
}

impl Trait for i8 { type AssociatedType = &'static str; }

foo(3_i8);
```

Here is that same example again, with some explanatory comments:

```ignore
trait Trait { type AssociatedType; }

fn foo<T>(t: T) where T: Trait<AssociatedType=u32> {
//                    ~~~~~~~~ ~~~~~~~~~~~~~~~~~~
//                        |            |
//         This says `foo` can         |
//           only be used with         |
//              some type that         |
//         implements `Trait`.         |
//                                     |
//                             This says not only must
//                             `T` be an impl of `Trait`
//                             but also that the impl
//                             must assign the type `u32`
//                             to the associated type.
    println!("in foo");
}

impl Trait for i8 { type AssociatedType = &'static str; }
~~~~~~~~~~~~~~~~~   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//      |                             |
// `i8` does have                     |
// implementation                     |
// of `Trait`...                      |
//                     ... but it is an implementation
//                     that assigns `&'static str` to
//                     the associated type.

foo(3_i8);
// Here, we invoke `foo` with an `i8`, which does not satisfy
// the constraint `<i8 as Trait>::AssociatedType=u32`, and
// therefore the type-checker complains with this error code.
```

Here is a more subtle instance of the same problem, that can
arise with for-loops in Rust:

```compile_fail
let vs: Vec<i32> = vec![1, 2, 3, 4];
for v in &vs {
    match v {
        1 => {},
        _ => {},
    }
}
```

The above fails because of an analogous type mismatch,
though may be harder to see. Again, here are some
explanatory comments for the same example:

```ignore
{
    let vs = vec![1, 2, 3, 4];

    // `for`-loops use a protocol based on the `Iterator`
    // trait. Each item yielded in a `for` loop has the
    // type `Iterator::Item` -- that is, `Item` is the
    // associated type of the concrete iterator impl.
    for v in &vs {
//      ~    ~~~
//      |     |
//      |    We borrow `vs`, iterating over a sequence of
//      |    *references* of type `&Elem` (where `Elem` is
//      |    vector's element type). Thus, the associated
//      |    type `Item` must be a reference `&`-type ...
//      |
//  ... and `v` has the type `Iterator::Item`, as dictated by
//  the `for`-loop protocol ...

        match v {
            1 => {}
//          ~
//          |
// ... but *here*, `v` is forced to have some integral type;
// only types like `u8`,`i8`,`u16`,`i16`, et cetera can
// match the pattern `1` ...

            _ => {}
        }

// ... therefore, the compiler complains, because it sees
// an attempt to solve the equations
// `some integral-type` = type-of-`v`
//                      = `Iterator::Item`
//                      = `&Elem` (i.e. `some reference type`)
//
// which cannot possibly all be true.

    }
}
```

To avoid those issues, you have to make the types match correctly.
So we can fix the previous examples like this:

```
// Basic Example:
trait Trait { type AssociatedType; }

fn foo<T>(t: T) where T: Trait<AssociatedType = &'static str> {
    println!("in foo");
}

impl Trait for i8 { type AssociatedType = &'static str; }

foo(3_i8);

// For-Loop Example:
let vs = vec![1, 2, 3, 4];
for v in &vs {
    match v {
        &1 => {}
        _ => {}
    }
}
```
"##,

E0272: r##"
The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
message for when a particular trait isn't implemented on a type placed in a
position that needs that trait. For example, when the following code is
compiled:

```compile_fail
fn foo<T: Index<u8>>(x: T){}

#[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
trait Index<Idx> { /* ... */ }

foo(true); // `bool` does not implement `Index<u8>`
```

There will be an error about `bool` not implementing `Index<u8>`, followed by a
note saying "the type `bool` cannot be indexed by `u8`".

As you can see, you can specify type parameters in curly braces for
substitution with the actual types (using the regular format string syntax) in
a given situation. Furthermore, `{Self}` will substitute to the type (in this
case, `bool`) that we tried to use.

This error appears when the curly braces contain an identifier which doesn't
match with any of the type parameters or the string `Self`. This might happen
if you misspelled a type parameter, or if you intended to use literal curly
braces. If it is the latter, escape the curly braces with a second curly brace
of the same type; e.g. a literal `{` is `{{`.
"##,

E0273: r##"
The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
message for when a particular trait isn't implemented on a type placed in a
position that needs that trait. For example, when the following code is
compiled:

```compile_fail
fn foo<T: Index<u8>>(x: T){}

#[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
trait Index<Idx> { /* ... */ }

foo(true); // `bool` does not implement `Index<u8>`
```

there will be an error about `bool` not implementing `Index<u8>`, followed by a
note saying "the type `bool` cannot be indexed by `u8`".

As you can see, you can specify type parameters in curly braces for
substitution with the actual types (using the regular format string syntax) in
a given situation. Furthermore, `{Self}` will substitute to the type (in this
case, `bool`) that we tried to use.

This error appears when the curly braces do not contain an identifier. Please
add one of the same name as a type parameter. If you intended to use literal
braces, use `{{` and `}}` to escape them.
"##,

E0274: r##"
The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
message for when a particular trait isn't implemented on a type placed in a
position that needs that trait. For example, when the following code is
compiled:

```compile_fail
fn foo<T: Index<u8>>(x: T){}

#[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
trait Index<Idx> { /* ... */ }

foo(true); // `bool` does not implement `Index<u8>`
```

there will be an error about `bool` not implementing `Index<u8>`, followed by a
note saying "the type `bool` cannot be indexed by `u8`".

For this to work, some note must be specified. An empty attribute will not do
anything, please remove the attribute or add some helpful note for users of the
trait.
"##,

E0275: r##"
This error occurs when there was a recursive trait requirement that overflowed
before it could be evaluated. Often this means that there is unbounded
recursion in resolving some type bounds.

For example, in the following code:

```compile_fail
trait Foo {}

struct Bar<T>(T);

impl<T> Foo for T where Bar<T>: Foo {}
```

To determine if a `T` is `Foo`, we need to check if `Bar<T>` is `Foo`. However,
to do this check, we need to determine that `Bar<Bar<T>>` is `Foo`. To
determine this, we check if `Bar<Bar<Bar<T>>>` is `Foo`, and so on. This is
clearly a recursive requirement that can't be resolved directly.

Consider changing your trait bounds so that they're less self-referential.
"##,

E0276: r##"
This error occurs when a bound in an implementation of a trait does not match
the bounds specified in the original trait. For example:

```compile_fail
trait Foo {
    fn foo<T>(x: T);
}

impl Foo for bool {
    fn foo<T>(x: T) where T: Copy {}
}
```

Here, all types implementing `Foo` must have a method `foo<T>(x: T)` which can
take any type `T`. However, in the `impl` for `bool`, we have added an extra
bound that `T` is `Copy`, which isn't compatible with the original trait.

Consider removing the bound from the method or adding the bound to the original
method definition in the trait.
"##,

E0277: r##"
You tried to use a type which doesn't implement some trait in a place which
expected that trait. Erroneous code example:

```compile_fail
// here we declare the Foo trait with a bar method
trait Foo {
    fn bar(&self);
}

// we now declare a function which takes an object implementing the Foo trait
fn some_func<T: Foo>(foo: T) {
    foo.bar();
}

fn main() {
    // we now call the method with the i32 type, which doesn't implement
    // the Foo trait
    some_func(5i32); // error: the trait `Foo` is not implemented for the
                     //        type `i32`
}
```

In order to fix this error, verify that the type you're using does implement
the trait. Example:

```
trait Foo {
    fn bar(&self);
}

fn some_func<T: Foo>(foo: T) {
    foo.bar(); // we can now use this method since i32 implements the
               // Foo trait
}

// we implement the trait on the i32 type
impl Foo for i32 {
    fn bar(&self) {}
}

fn main() {
    some_func(5i32); // ok!
}
```
"##,

E0281: r##"
You tried to supply a type which doesn't implement some trait in a location
which expected that trait. This error typically occurs when working with
`Fn`-based types. Erroneous code example:

```compile_fail
fn foo<F: Fn()>(x: F) { }

fn main() {
    // type mismatch: the type ... implements the trait `core::ops::Fn<(_,)>`,
    // but the trait `core::ops::Fn<()>` is required (expected (), found tuple
    // [E0281]
    foo(|y| { });
}
```

The issue in this case is that `foo` is defined as accepting a `Fn` with no
arguments, but the closure we attempted to pass to it requires one argument.
"##,

E0282: r##"
This error indicates that type inference did not result in one unique possible
type, and extra information is required. In most cases this can be provided
by adding a type annotation. Sometimes you need to specify a generic type
parameter manually.

A common example is the `collect` method on `Iterator`. It has a generic type
parameter with a `FromIterator` bound, which for a `char` iterator is
implemented by `Vec` and `String` among others. Consider the following snippet
that reverses the characters of a string:

```compile_fail
let x = "hello".chars().rev().collect();
```

In this case, the compiler cannot infer what the type of `x` should be:
`Vec<char>` and `String` are both suitable candidates. To specify which type to
use, you can use a type annotation on `x`:

```
let x: Vec<char> = "hello".chars().rev().collect();
```

It is not necessary to annotate the full type. Once the ambiguity is resolved,
the compiler can infer the rest:

```
let x: Vec<_> = "hello".chars().rev().collect();
```

Another way to provide the compiler with enough information, is to specify the
generic type parameter:

```
let x = "hello".chars().rev().collect::<Vec<char>>();
```

Again, you need not specify the full type if the compiler can infer it:

```
let x = "hello".chars().rev().collect::<Vec<_>>();
```

Apart from a method or function with a generic type parameter, this error can
occur when a type parameter of a struct or trait cannot be inferred. In that
case it is not always possible to use a type annotation, because all candidates
have the same return type. For instance:

```compile_fail
struct Foo<T> {
    num: T,
}

impl<T> Foo<T> {
    fn bar() -> i32 {
        0
    }

    fn baz() {
        let number = Foo::bar();
    }
}
```

This will fail because the compiler does not know which instance of `Foo` to
call `bar` on. Change `Foo::bar()` to `Foo::<T>::bar()` to resolve the error.
"##,

E0283: r##"
This error occurs when the compiler doesn't have enough information
to unambiguously choose an implementation.

For example:

```compile_fail
trait Generator {
    fn create() -> u32;
}

struct Impl;

impl Generator for Impl {
    fn create() -> u32 { 1 }
}

struct AnotherImpl;

impl Generator for AnotherImpl {
    fn create() -> u32 { 2 }
}

fn main() {
    let cont: u32 = Generator::create();
    // error, impossible to choose one of Generator trait implementation
    // Impl or AnotherImpl? Maybe anything else?
}
```

To resolve this error use the concrete type:

```
trait Generator {
    fn create() -> u32;
}

struct AnotherImpl;

impl Generator for AnotherImpl {
    fn create() -> u32 { 2 }
}

fn main() {
    let gen1 = AnotherImpl::create();

    // if there are multiple methods with same name (different traits)
    let gen2 = <AnotherImpl as Generator>::create();
}
```
"##,

E0296: r##"
This error indicates that the given recursion limit could not be parsed. Ensure
that the value provided is a positive integer between quotes, like so:

```
#![recursion_limit="1000"]
```
"##,

E0297: r##"
Patterns used to bind names must be irrefutable. That is, they must guarantee
that a name will be extracted in all cases. Instead of pattern matching the
loop variable, consider using a `match` or `if let` inside the loop body. For
instance:

```compile_fail
let xs : Vec<Option<i32>> = vec!(Some(1), None);

// This fails because `None` is not covered.
for Some(x) in xs {
    // ...
}
```

Match inside the loop instead:

```
let xs : Vec<Option<i32>> = vec!(Some(1), None);

for item in xs {
    match item {
        Some(x) => {},
        None => {},
    }
}
```

Or use `if let`:

```
let xs : Vec<Option<i32>> = vec!(Some(1), None);

for item in xs {
    if let Some(x) = item {
        // ...
    }
}
```
"##,

E0301: r##"
Mutable borrows are not allowed in pattern guards, because matching cannot have
side effects. Side effects could alter the matched object or the environment
on which the match depends in such a way, that the match would not be
exhaustive. For instance, the following would not match any arm if mutable
borrows were allowed:

```compile_fail
match Some(()) {
    None => { },
    option if option.take().is_none() => {
        /* impossible, option is `Some` */
    },
    Some(_) => { } // When the previous match failed, the option became `None`.
}
```
"##,

E0302: r##"
Assignments are not allowed in pattern guards, because matching cannot have
side effects. Side effects could alter the matched object or the environment
on which the match depends in such a way, that the match would not be
exhaustive. For instance, the following would not match any arm if assignments
were allowed:

```compile_fail
match Some(()) {
    None => { },
    option if { option = None; false } { },
    Some(_) => { } // When the previous match failed, the option became `None`.
}
```
"##,

E0303: r##"
In certain cases it is possible for sub-bindings to violate memory safety.
Updates to the borrow checker in a future version of Rust may remove this
restriction, but for now patterns must be rewritten without sub-bindings.

```ignore
// Before.
match Some("hi".to_string()) {
    ref op_string_ref @ Some(s) => {},
    None => {},
}

// After.
match Some("hi".to_string()) {
    Some(ref s) => {
        let op_string_ref = &Some(s);
        // ...
    },
    None => {},
}
```

The `op_string_ref` binding has type `&Option<&String>` in both cases.

See also https://github.com/rust-lang/rust/issues/14587
"##,

E0306: r##"
In an array literal `[x; N]`, `N` is the number of elements in the array. This
must be an unsigned integer. Erroneous code example:

```compile_fail
let x = [0i32; true]; // error: expected positive integer for repeat count,
                      //        found boolean
```

Working example:

```
let x = [0i32; 2];
```
"##,

E0307: r##"
The length of an array is part of its type. For this reason, this length must
be a compile-time constant. Erroneous code example:

```compile_fail
    let len = 10;
    let x = [0i32; len]; // error: expected constant integer for repeat count,
                         //        found variable
```
"##,

E0308: r##"
This error occurs when the compiler was unable to infer the concrete type of a
variable. It can occur for several cases, the most common of which is a
mismatch in the expected type that the compiler inferred for a variable's
initializing expression, and the actual type explicitly assigned to the
variable.

For example:

```compile_fail
let x: i32 = "I am not a number!";
//     ~~~   ~~~~~~~~~~~~~~~~~~~~
//      |             |
//      |    initializing expression;
//      |    compiler infers type `&str`
//      |
//    type `i32` assigned to variable `x`
```

Another situation in which this occurs is when you attempt to use the `try!`
macro inside a function that does not return a `Result<T, E>`:

```compile_fail
use std::fs::File;

fn main() {
    let mut f = try!(File::create("foo.txt"));
}
```

This code gives an error like this:

```text
<std macros>:5:8: 6:42 error: mismatched types:
 expected `()`,
     found `core::result::Result<_, _>`
 (expected (),
     found enum `core::result::Result`) [E0308]
```

`try!` returns a `Result<T, E>`, and so the function must. But `main()` has
`()` as its return type, hence the error.
"##,

E0309: r##"
Types in type definitions have lifetimes associated with them that represent
how long the data stored within them is guaranteed to be live. This lifetime
must be as long as the data needs to be alive, and missing the constraint that
denotes this will cause this error.

```compile_fail
// This won't compile because T is not constrained, meaning the data
// stored in it is not guaranteed to last as long as the reference
struct Foo<'a, T> {
    foo: &'a T
}
```

This will compile, because it has the constraint on the type parameter:

```
struct Foo<'a, T: 'a> {
    foo: &'a T
}
```
"##,

E0310: r##"
Types in type definitions have lifetimes associated with them that represent
how long the data stored within them is guaranteed to be live. This lifetime
must be as long as the data needs to be alive, and missing the constraint that
denotes this will cause this error.

```compile_fail
// This won't compile because T is not constrained to the static lifetime
// the reference needs
struct Foo<T> {
    foo: &'static T
}

This will compile, because it has the constraint on the type parameter:

```
struct Foo<T: 'static> {
    foo: &'static T
}
```
"##,

E0398: r##"
In Rust 1.3, the default object lifetime bounds are expected to change, as
described in RFC #1156 [1]. You are getting a warning because the compiler
thinks it is possible that this change will cause a compilation error in your
code. It is possible, though unlikely, that this is a false alarm.

The heart of the change is that where `&'a Box<SomeTrait>` used to default to
`&'a Box<SomeTrait+'a>`, it now defaults to `&'a Box<SomeTrait+'static>` (here,
`SomeTrait` is the name of some trait type). Note that the only types which are
affected are references to boxes, like `&Box<SomeTrait>` or
`&[Box<SomeTrait>]`. More common types like `&SomeTrait` or `Box<SomeTrait>`
are unaffected.

To silence this warning, edit your code to use an explicit bound. Most of the
time, this means that you will want to change the signature of a function that
you are calling. For example, if the error is reported on a call like `foo(x)`,
and `foo` is defined as follows:

```ignore
fn foo(arg: &Box<SomeTrait>) { ... }
```

You might change it to:

```ignore
fn foo<'a>(arg: &Box<SomeTrait+'a>) { ... }
