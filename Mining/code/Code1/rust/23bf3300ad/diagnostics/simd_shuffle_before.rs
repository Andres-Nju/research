    fn simd_shuffle<A,B>(a: A, b: A, c: [u32; 8]) -> B;
    // error: invalid `simd_shuffle`, needs length: `simd_shuffle`
}
```

The `simd_shuffle` function needs the length of the array passed as
last parameter in its name. Example:

```
#![feature(platform_intrinsics)]

extern "platform-intrinsic" {
    fn simd_shuffle8<A,B>(a: A, b: A, c: [u32; 8]) -> B;
}
```
"##,

E0440: r##"
A platform-specific intrinsic function has the wrong number of type
parameters. Erroneous code example:

```compile_fail,E0440
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct f64x2(f64, f64);

extern "platform-intrinsic" {
    fn x86_mm_movemask_pd<T>(x: f64x2) -> i32;
    // error: platform-specific intrinsic has wrong number of type
    //        parameters
}
```

Please refer to the function declaration to see if it corresponds
with yours. Example:

```
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct f64x2(f64, f64);

extern "platform-intrinsic" {
    fn x86_mm_movemask_pd(x: f64x2) -> i32;
}
```
"##,

E0441: r##"
An unknown platform-specific intrinsic function was used. Erroneous
code example:

```compile_fail,E0441
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);

extern "platform-intrinsic" {
    fn x86_mm_adds_ep16(x: i16x8, y: i16x8) -> i16x8;
    // error: unrecognized platform-specific intrinsic function
}
```

Please verify that the function name wasn't misspelled, and ensure
that it is declared in the rust source code (in the file
src/librustc_platform_intrinsics/x86.rs). Example:

```
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);

extern "platform-intrinsic" {
    fn x86_mm_adds_epi16(x: i16x8, y: i16x8) -> i16x8; // ok!
}
```
"##,

E0442: r##"
Intrinsic argument(s) and/or return value have the wrong type.
Erroneous code example:

```compile_fail,E0442
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i8x16(i8, i8, i8, i8, i8, i8, i8, i8,
             i8, i8, i8, i8, i8, i8, i8, i8);
#[repr(simd)]
struct i32x4(i32, i32, i32, i32);
#[repr(simd)]
struct i64x2(i64, i64);

extern "platform-intrinsic" {
    fn x86_mm_adds_epi16(x: i8x16, y: i32x4) -> i64x2;
    // error: intrinsic arguments/return value have wrong type
}
```

To fix this error, please refer to the function declaration to give
it the awaited types. Example:

```
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);

extern "platform-intrinsic" {
    fn x86_mm_adds_epi16(x: i16x8, y: i16x8) -> i16x8; // ok!
}
```
"##,

E0443: r##"
Intrinsic argument(s) and/or return value have the wrong type.
Erroneous code example:

```compile_fail,E0443
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);
#[repr(simd)]
struct i64x8(i64, i64, i64, i64, i64, i64, i64, i64);

extern "platform-intrinsic" {
    fn x86_mm_adds_epi16(x: i16x8, y: i16x8) -> i64x8;
    // error: intrinsic argument/return value has wrong type
}
```

To fix this error, please refer to the function declaration to give
it the awaited types. Example:

```
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);

extern "platform-intrinsic" {
    fn x86_mm_adds_epi16(x: i16x8, y: i16x8) -> i16x8; // ok!
}
```
"##,

E0444: r##"
A platform-specific intrinsic function has wrong number of arguments.
Erroneous code example:

```compile_fail,E0444
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct f64x2(f64, f64);

extern "platform-intrinsic" {
    fn x86_mm_movemask_pd(x: f64x2, y: f64x2, z: f64x2) -> i32;
    // error: platform-specific intrinsic has invalid number of arguments
}
```

Please refer to the function declaration to see if it corresponds
with yours. Example:

```
#![feature(repr_simd)]
#![feature(platform_intrinsics)]

#[repr(simd)]
struct f64x2(f64, f64);

extern "platform-intrinsic" {
    fn x86_mm_movemask_pd(x: f64x2) -> i32; // ok!
}
```
"##,

E0516: r##"
The `typeof` keyword is currently reserved but unimplemented.
Erroneous code example:

```compile_fail,E0516
fn main() {
    let x: typeof(92) = 92;
}
```

Try using type inference instead. Example:

```
fn main() {
    let x = 92;
}
```
"##,

E0520: r##"
A non-default implementation was already made on this type so it cannot be
specialized further. Erroneous code example:

```compile_fail,E0520
#![feature(specialization)]

trait SpaceLlama {
    fn fly(&self);
}

// applies to all T
impl<T> SpaceLlama for T {
    default fn fly(&self) {}
}

// non-default impl
// applies to all `Clone` T and overrides the previous impl
impl<T: Clone> SpaceLlama for T {
    fn fly(&self) {}
}

// since `i32` is clone, this conflicts with the previous implementation
impl SpaceLlama for i32 {
    default fn fly(&self) {}
    // error: item `fly` is provided by an `impl` that specializes
    //        another, but the item in the parent `impl` is not marked
    //        `default` and so it cannot be specialized.
}
```

Specialization only allows you to override `default` functions in
implementations.

To fix this error, you need to mark all the parent implementations as default.
Example:

```
#![feature(specialization)]

trait SpaceLlama {
    fn fly(&self);
}

// applies to all T
impl<T> SpaceLlama for T {
    default fn fly(&self) {} // This is a parent implementation.
}

// applies to all `Clone` T; overrides the previous impl
impl<T: Clone> SpaceLlama for T {
    default fn fly(&self) {} // This is a parent implementation but was
                             // previously not a default one, causing the error
}

// applies to i32, overrides the previous two impls
impl SpaceLlama for i32 {
    fn fly(&self) {} // And now that's ok!
}
```
"##,

E0527: r##"
The number of elements in an array or slice pattern differed from the number of
elements in the array being matched.

Example of erroneous code:

```compile_fail,E0527
#![feature(slice_patterns)]

let r = &[1, 2, 3, 4];
match r {
    &[a, b] => { // error: pattern requires 2 elements but array
                 //        has 4
        println!("a={}, b={}", a, b);
    }
}
```

Ensure that the pattern is consistent with the size of the matched
array. Additional elements can be matched with `..`:

```
#![feature(slice_patterns)]

let r = &[1, 2, 3, 4];
match r {
    &[a, b, ..] => { // ok!
        println!("a={}, b={}", a, b);
    }
}
```
"##,

E0528: r##"
An array or slice pattern required more elements than were present in the
matched array.

Example of erroneous code:

```compile_fail,E0528
#![feature(slice_patterns)]

let r = &[1, 2];
match r {
    &[a, b, c, rest..] => { // error: pattern requires at least 3
                            //        elements but array has 2
        println!("a={}, b={}, c={} rest={:?}", a, b, c, rest);
    }
}
```

Ensure that the matched array has at least as many elements as the pattern
requires. You can match an arbitrary number of remaining elements with `..`:

```
#![feature(slice_patterns)]

let r = &[1, 2, 3, 4, 5];
match r {
    &[a, b, c, rest..] => { // ok!
        // prints `a=1, b=2, c=3 rest=[4, 5]`
        println!("a={}, b={}, c={} rest={:?}", a, b, c, rest);
    }
}
```
"##,

E0529: r##"
An array or slice pattern was matched against some other type.

Example of erroneous code:

```compile_fail,E0529
#![feature(slice_patterns)]

let r: f32 = 1.0;
match r {
    [a, b] => { // error: expected an array or slice, found `f32`
        println!("a={}, b={}", a, b);
    }
}
```

Ensure that the pattern and the expression being matched on are of consistent
types:

```
#![feature(slice_patterns)]

let r = [1.0, 2.0];
match r {
    [a, b] => { // ok!
        println!("a={}, b={}", a, b);
    }
}
```
"##,

E0559: r##"
An unknown field was specified into an enum's structure variant.

Erroneous code example:

```compile_fail,E0559
enum Field {
    Fool { x: u32 },
}

let s = Field::Fool { joke: 0 };
// error: struct variant `Field::Fool` has no field named `joke`
```

Verify you didn't misspell the field's name or that the field exists. Example:

```
enum Field {
    Fool { joke: u32 },
}

let s = Field::Fool { joke: 0 }; // ok!
```
"##,

E0560: r##"
An unknown field was specified into a structure.

Erroneous code example:

```compile_fail,E0560
struct Simba {
    mother: u32,
}

let s = Simba { mother: 1, father: 0 };
// error: structure `Simba` has no field named `father`
```

Verify you didn't misspell the field's name or that the field exists. Example:

```
struct Simba {
    mother: u32,
    father: u32,
}

let s = Simba { mother: 1, father: 0 }; // ok!
```
"##,

E0562: r##"
Abstract return types (written `impl Trait` for some trait `Trait`) are only
allowed as function return types.

Erroneous code example:

```compile_fail,E0562
#![feature(conservative_impl_trait)]

fn main() {
    let count_to_ten: impl Iterator<Item=usize> = 0..10;
    // error: `impl Trait` not allowed outside of function and inherent method
    //        return types
    for i in count_to_ten {
        println!("{}", i);
    }
}
```

Make sure `impl Trait` only appears in return-type position.

```
#![feature(conservative_impl_trait)]

fn count_to_n(n: usize) -> impl Iterator<Item=usize> {
    0..n
}

fn main() {
    for i in count_to_n(10) {  // ok!
        println!("{}", i);
    }
}
```

See [RFC 1522] for more details.

[RFC 1522]: https://github.com/rust-lang/rfcs/blob/master/text/1522-conservative-impl-trait.md
"##,

E0569: r##"
If an impl has a generic parameter with the `#[may_dangle]` attribute, then
that impl must be declared as an `unsafe impl.

Erroneous code example:

```compile_fail,E0569
#![feature(generic_param_attrs)]
#![feature(dropck_eyepatch)]

struct Foo<X>(X);
impl<#[may_dangle] X> Drop for Foo<X> {
    fn drop(&mut self) { }
}
```

In this example, we are asserting that the destructor for `Foo` will not
access any data of type `X`, and require this assertion to be true for
overall safety in our program. The compiler does not currently attempt to
verify this assertion; therefore we must tag this `impl` as unsafe.
"##,

E0570: r##"
The requested ABI is unsupported by the current target.

The rust compiler maintains for each target a blacklist of ABIs unsupported on
that target. If an ABI is present in such a list this usually means that the
target / ABI combination is currently unsupported by llvm.

If necessary, you can circumvent this check using custom target specifications.
"##,

E0572: r##"
A return statement was found outside of a function body.

Erroneous code example:

```compile_fail,E0572
const FOO: u32 = return 0; // error: return statement outside of function body

fn main() {}
```

To fix this issue, just remove the return keyword or move the expression into a
function. Example:

```
const FOO: u32 = 0;

fn some_fn() -> u32 {
    return FOO;
}

fn main() {
    some_fn();
}
```
"##,

E0581: r##"
In a `fn` type, a lifetime appears only in the return type,
and not in the arguments types.

Erroneous code example:

```compile_fail,E0581
fn main() {
    // Here, `'a` appears only in the return type:
    let x: for<'a> fn() -> &'a i32;
}
```

To fix this issue, either use the lifetime in the arguments, or use
`'static`. Example:

```
fn main() {
    // Here, `'a` appears only in the return type:
    let x: for<'a> fn(&'a i32) -> &'a i32;
    let y: fn() -> &'static i32;
}
```

Note: The examples above used to be (erroneously) accepted by the
compiler, but this was since corrected. See [issue #33685] for more
details.

[issue #33685]: https://github.com/rust-lang/rust/issues/33685
"##,

E0582: r##"
A lifetime appears only in an associated-type binding,
and not in the input types to the trait.

Erroneous code example:

```compile_fail,E0582
fn bar<F>(t: F)
    // No type can satisfy this requirement, since `'a` does not
    // appear in any of the input types (here, `i32`):
    where F: for<'a> Fn(i32) -> Option<&'a i32>
{
}

fn main() { }
```

To fix this issue, either use the lifetime in the inputs, or use
`'static`. Example:

```
fn bar<F, G>(t: F, u: G)
    where F: for<'a> Fn(&'a i32) -> Option<&'a i32>,
          G: Fn(i32) -> Option<&'static i32>,
{
}

fn main() { }
```

Note: The examples above used to be (erroneously) accepted by the
compiler, but this was since corrected. See [issue #33685] for more
details.

[issue #33685]: https://github.com/rust-lang/rust/issues/33685
"##,

E0599: r##"
```compile_fail,E0599
struct Mouth;

let x = Mouth;
x.chocolate(); // error: no method named `chocolate` found for type `Mouth`
               //        in the current scope
```
"##,

E0600: r##"
An unary operator was used on a type which doesn't implement it.

Example of erroneous code:

```compile_fail,E0600
enum Question {
    Yes,
    No,
}

!Question::Yes; // error: cannot apply unary operator `!` to type `Question`
```

In this case, `Question` would need to implement the `std::ops::Not` trait in
order to be able to use `!` on it. Let's implement it:

```
use std::ops::Not;

enum Question {
    Yes,
    No,
}

// We implement the `Not` trait on the enum.
impl Not for Question {
    type Output = bool;

    fn not(self) -> bool {
        match self {
            Question::Yes => false, // If the `Answer` is `Yes`, then it
                                    // returns false.
            Question::No => true, // And here we do the opposite.
        }
    }
}

assert_eq!(!Question::Yes, false);
assert_eq!(!Question::No, true);
```
"##,

E0608: r##"
An attempt to index into a type which doesn't implement the `std::ops::Index`
trait was performed.

Erroneous code example:

```compile_fail,E0608
0u8[2]; // error: cannot index into a value of type `u8`
```

To be able to index into a type it needs to implement the `std::ops::Index`
trait. Example:

```
let v: Vec<u8> = vec![0, 1, 2, 3];

// The `Vec` type implements the `Index` trait so you can do:
println!("{}", v[2]);
```
"##,

E0604: r##"
A cast to `char` was attempted on a type other than `u8`.

Erroneous code example:

```compile_fail,E0604
0u32 as char; // error: only `u8` can be cast as `char`, not `u32`
```

As the error message indicates, only `u8` can be cast into `char`. Example:

```
let c = 86u8 as char; // ok!
assert_eq!(c, 'V');
```

For more information about casts, take a look at The Book:
https://doc.rust-lang.org/book/first-edition/casting-between-types.html
"##,

E0605: r##"
An invalid cast was attempted.

Erroneous code examples:

```compile_fail,E0605
let x = 0u8;
x as Vec<u8>; // error: non-primitive cast: `u8` as `std::vec::Vec<u8>`

// Another example

let v = 0 as *const u8; // So here, `v` is a `*const u8`.
v as &u8; // error: non-primitive cast: `*const u8` as `&u8`
```

Only primitive types can be cast into each other. Examples:

```
let x = 0u8;
x as u32; // ok!

let v = 0 as *const u8;
v as *const i8; // ok!
```

For more information about casts, take a look at The Book:
https://doc.rust-lang.org/book/first-edition/casting-between-types.html
"##,

E0606: r##"
An incompatible cast was attempted.

Erroneous code example:

```compile_fail,E0606
let x = &0u8; // Here, `x` is a `&u8`.
let y: u32 = x as u32; // error: casting `&u8` as `u32` is invalid
```

When casting, keep in mind that only primitive types can be cast into each
other. Example:

```
let x = &0u8;
let y: u32 = *x as u32; // We dereference it first and then cast it.
```

For more information about casts, take a look at The Book:
https://doc.rust-lang.org/book/first-edition/casting-between-types.html
"##,

E0607: r##"
A cast between a thin and a fat pointer was attempted.

Erroneous code example:

```compile_fail,E0607
let v = 0 as *const u8;
v as *const [u8];
```

First: what are thin and fat pointers?

Thin pointers are "simple" pointers: they are purely a reference to a memory
address.

Fat pointers are pointers referencing Dynamically Sized Types (also called DST).
DST don't have a statically known size, therefore they can only exist behind
some kind of pointers that contain additional information. Slices and trait
objects are DSTs. In the case of slices, the additional information the fat
pointer holds is their size.

To fix this error, don't try to cast directly between thin and fat pointers.

For more information about casts, take a look at The Book:
https://doc.rust-lang.org/book/first-edition/casting-between-types.html
"##,

E0609: r##"
Attempted to access a non-existent field in a struct.

Erroneous code example:

```compile_fail,E0609
struct StructWithFields {
    x: u32,
}

let s = StructWithFields { x: 0 };
println!("{}", s.foo); // error: no field `foo` on type `StructWithFields`
```

To fix this error, check that you didn't misspell the field's name or that the
field actually exists. Example:

```
struct StructWithFields {
    x: u32,
}

let s = StructWithFields { x: 0 };
println!("{}", s.x); // ok!
```
"##,

E0610: r##"
Attempted to access a field on a primitive type.

Erroneous code example:

```compile_fail,E0610
let x: u32 = 0;
println!("{}", x.foo); // error: `{integer}` is a primitive type, therefore
                       //        doesn't have fields
```

Primitive types are the most basic types available in Rust and don't have
fields. To access data via named fields, struct types are used. Example:

```
// We declare struct called `Foo` containing two fields:
struct Foo {
    x: u32,
    y: i64,
}

// We create an instance of this struct:
let variable = Foo { x: 0, y: -12 };
// And we can now access its fields:
println!("x: {}, y: {}", variable.x, variable.y);
```

For more information about primitives and structs, take a look at The Book:
https://doc.rust-lang.org/book/first-edition/primitive-types.html
https://doc.rust-lang.org/book/first-edition/structs.html
"##,

E0611: r##"
Attempted to access a private field on a tuple-struct.

Erroneous code example:

```compile_fail,E0611
mod some_module {
    pub struct Foo(u32);

    impl Foo {
        pub fn new() -> Foo { Foo(0) }
    }
}

let y = some_module::Foo::new();
println!("{}", y.0); // error: field `0` of tuple-struct `some_module::Foo`
                     //        is private
```

Since the field is private, you have two solutions:

1) Make the field public:

```
mod some_module {
    pub struct Foo(pub u32); // The field is now public.

    impl Foo {
        pub fn new() -> Foo { Foo(0) }
    }
}

let y = some_module::Foo::new();
println!("{}", y.0); // So we can access it directly.
```

2) Add a getter function to keep the field private but allow for accessing its
value:

```
mod some_module {
    pub struct Foo(u32);

    impl Foo {
        pub fn new() -> Foo { Foo(0) }

        // We add the getter function.
        pub fn get(&self) -> &u32 { &self.0 }
    }
}

let y = some_module::Foo::new();
println!("{}", y.get()); // So we can get the value through the function.
```
"##,

E0612: r##"
Attempted out-of-bounds tuple index.

Erroneous code example:

```compile_fail,E0612
struct Foo(u32);

let y = Foo(0);
println!("{}", y.1); // error: attempted out-of-bounds tuple index `1`
                     //        on type `Foo`
```

If a tuple/tuple-struct type has n fields, you can only try to access these n
fields from 0 to (n - 1). So in this case, you can only index `0`. Example:

```
struct Foo(u32);

let y = Foo(0);
println!("{}", y.0); // ok!
```
"##,

E0614: r##"
Attempted to dereference a variable which cannot be dereferenced.

Erroneous code example:

```compile_fail,E0614
let y = 0u32;
*y; // error: type `u32` cannot be dereferenced
```

Only types implementing `std::ops::Deref` can be dereferenced (such as `&T`).
Example:

```
let y = 0u32;
let x = &y;
// So here, `x` is a `&u32`, so we can dereference it:
*x; // ok!
```
"##,

E0615: r##"
Attempted to access a method like a field.

Erroneous code example:

```compile_fail,E0615
struct Foo {
    x: u32,
}

impl Foo {
    fn method(&self) {}
}

let f = Foo { x: 0 };
f.method; // error: attempted to take value of method `method` on type `Foo`
```

If you want to use a method, add `()` after it:

```
# struct Foo { x: u32 }
# impl Foo { fn method(&self) {} }
# let f = Foo { x: 0 };
f.method();
```

However, if you wanted to access a field of a struct check that the field name
is spelled correctly. Example:

```
# struct Foo { x: u32 }
# impl Foo { fn method(&self) {} }
# let f = Foo { x: 0 };
println!("{}", f.x);
```
"##,

E0616: r##"
Attempted to access a private field on a struct.

Erroneous code example:

```compile_fail,E0616
mod some_module {
    pub struct Foo {
        x: u32, // So `x` is private in here.
    }

    impl Foo {
        pub fn new() -> Foo { Foo { x: 0 } }
    }
}

let f = some_module::Foo::new();
println!("{}", f.x); // error: field `x` of struct `some_module::Foo` is private
```

If you want to access this field, you have two options:

1) Set the field public:

```
mod some_module {
    pub struct Foo {
        pub x: u32, // `x` is now public.
    }

    impl Foo {
        pub fn new() -> Foo { Foo { x: 0 } }
    }
}

let f = some_module::Foo::new();
println!("{}", f.x); // ok!
```

2) Add a getter function:

```
mod some_module {
    pub struct Foo {
        x: u32, // So `x` is still private in here.
    }

    impl Foo {
        pub fn new() -> Foo { Foo { x: 0 } }

        // We create the getter function here:
        pub fn get_x(&self) -> &u32 { &self.x }
    }
}

let f = some_module::Foo::new();
println!("{}", f.get_x()); // ok!
```
"##,

E0617: r##"
Attempted to pass an invalid type of variable into a variadic function.

Erroneous code example:

```compile_fail,E0617
extern {
    fn printf(c: *const i8, ...);
}

unsafe {
    printf(::std::ptr::null(), 0f32);
    // error: can't pass an `f32` to variadic function, cast to `c_double`
}
```

Certain Rust types must be cast before passing them to a variadic function,
because of arcane ABI rules dictated by the C standard. To fix the error,
cast the value to the type specified by the error message (which you may need
to import from `std::os::raw`).
"##,

E0618: r##"
Attempted to call something which isn't a function nor a method.

Erroneous code examples:

```compile_fail,E0618
enum X {
    Entry,
}

X::Entry(); // error: expected function, found `X::Entry`

// Or even simpler:
let x = 0i32;
x(); // error: expected function, found `i32`
```

Only functions and methods can be called using `()`. Example:

```
// We declare a function:
fn i_am_a_function() {}

// And we call it:
i_am_a_function();
```
"##,

E0619: r##"
The type-checker needed to know the type of an expression, but that type had not
yet been inferred.

Erroneous code example:

```compile_fail,E0619
let mut x = vec![];
match x.pop() {
    Some(v) => {
        // Here, the type of `v` is not (yet) known, so we
        // cannot resolve this method call:
        v.to_uppercase(); // error: the type of this value must be known in
                          //        this context
    }
    None => {}
}
```

Type inference typically proceeds from the top of the function to the bottom,
figuring out types as it goes. In some cases -- notably method calls and
overloadable operators like `*` -- the type checker may not have enough
information *yet* to make progress. This can be true even if the rest of the
function provides enough context (because the type-checker hasn't looked that
far ahead yet). In this case, type annotations can be used to help it along.

To fix this error, just specify the type of the variable. Example:

```
let mut x: Vec<String> = vec![]; // We precise the type of the vec elements.
match x.pop() {
    Some(v) => {
        v.to_uppercase(); // Since rustc now knows the type of the vec elements,
                          // we can use `v`'s methods.
    }
    None => {}
}
```
"##,

E0620: r##"
A cast to an unsized type was attempted.

Erroneous code example:

```compile_fail,E0620
let x = &[1_usize, 2] as [usize]; // error: cast to unsized type: `&[usize; 2]`
                                  //        as `[usize]`
```

In Rust, some types don't have a known size at compile-time. For example, in a
slice type like `[u32]`, the number of elements is not known at compile-time and
hence the overall size cannot be computed. As a result, such types can only be
manipulated through a reference (e.g., `&T` or `&mut T`) or other pointer-type
(e.g., `Box` or `Rc`). Try casting to a reference instead:

```
let x = &[1_usize, 2] as &[usize]; // ok!
```
"##,

E0622: r##"
An intrinsic was declared without being a function.

Erroneous code example:

```compile_fail,E0622
#![feature(intrinsics)]
extern "rust-intrinsic" {
    pub static breakpoint : unsafe extern "rust-intrinsic" fn();
    // error: intrinsic must be a function
}

fn main() { unsafe { breakpoint(); } }
```

An intrinsic is a function available for use in a given programming language
whose implementation is handled specially by the compiler. In order to fix this
error, just declare a function.
"##,

E0624: r##"
A private item was used outside of its scope.

Erroneous code example:

```compile_fail,E0624
mod inner {
    pub struct Foo;

    impl Foo {
        fn method(&self) {}
    }
}

let foo = inner::Foo;
foo.method(); // error: method `method` is private
```

Two possibilities are available to solve this issue:

1. Only use the item in the scope it has been defined:

```
mod inner {
    pub struct Foo;

    impl Foo {
        fn method(&self) {}
    }

    pub fn call_method(foo: &Foo) { // We create a public function.
        foo.method(); // Which calls the item.
    }
}

let foo = inner::Foo;
inner::call_method(&foo); // And since the function is public, we can call the
                          // method through it.
```

2. Make the item public:

```
mod inner {
    pub struct Foo;

    impl Foo {
        pub fn method(&self) {} // It's now public.
    }
