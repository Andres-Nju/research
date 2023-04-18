File_Code/rust/6a2ef9a25f/diagnostics/diagnostics_after.rs --- Rust
 18 E0001: r##"                                                                                                                                               18 E0001: r##"
 19 ## Note: this error code is no longer emitted by the compiler.                                                                                            19 #### Note: this error code is no longer emitted by the compiler.
 20                                                                                                                                                           20 
 21 This error suggests that the expression arm corresponding to the noted pattern                                                                            21 This error suggests that the expression arm corresponding to the noted pattern
 22 will never be reached as for all possible values of the expression being                                                                                  22 will never be reached as for all possible values of the expression being
 23 matched, one of the preceding patterns will match.                                                                                                        23 matched, one of the preceding patterns will match.
 24                                                                                                                                                           24 
 25 This means that perhaps some of the preceding patterns are too general, this                                                                              25 This means that perhaps some of the preceding patterns are too general, this
 26 one is too specific or the ordering is incorrect.                                                                                                         26 one is too specific or the ordering is incorrect.
 27                                                                                                                                                           27 
 28 For example, the following `match` block has too many arms:                                                                                               28 For example, the following `match` block has too many arms:
 29                                                                                                                                                           29 
 30 ```                                                                                                                                                       30 ```
 31 match Some(0) {                                                                                                                                           31 match Some(0) {
 32     Some(bar) => {/* ... */}                                                                                                                              32     Some(bar) => {/* ... */}
 33     x => {/* ... */} // This handles the `None` case                                                                                                      33     x => {/* ... */} // This handles the `None` case
 34     _ => {/* ... */} // All possible cases have already been handled                                                                                      34     _ => {/* ... */} // All possible cases have already been handled
 35 }                                                                                                                                                         35 }
 36 ```                                                                                                                                                       36 ```
 37                                                                                                                                                           37 
 38 `match` blocks have their patterns matched in order, so, for example, putting                                                                             38 `match` blocks have their patterns matched in order, so, for example, putting
 39 a wildcard arm above a more specific arm will make the latter arm irrelevant.                                                                             39 a wildcard arm above a more specific arm will make the latter arm irrelevant.
 40                                                                                                                                                           40 
 41 Ensure the ordering of the match arm is correct and remove any superfluous                                                                                41 Ensure the ordering of the match arm is correct and remove any superfluous
 42 arms.                                                                                                                                                     42 arms.
 43 "##,                                                                                                                                                      43 "##,
 44                                                                                                                                                           44 
 45 E0002: r##"                                                                                                                                               45 E0002: r##"
 46 ## Note: this error code is no longer emitted by the compiler.                                                                                            46 #### Note: this error code is no longer emitted by the compiler.
 47                                                                                                                                                           47 
 48 This error indicates that an empty match expression is invalid because the type                                                                           48 This error indicates that an empty match expression is invalid because the type
 49 it is matching on is non-empty (there exist values of this type). In safe code                                                                            49 it is matching on is non-empty (there exist values of this type). In safe code
 50 it is impossible to create an instance of an empty type, so empty match                                                                                   50 it is impossible to create an instance of an empty type, so empty match
 51 expressions are almost never desired. This error is typically fixed by adding                                                                             51 expressions are almost never desired. This error is typically fixed by adding
 52 one or more cases to the match expression.                                                                                                                52 one or more cases to the match expression.
 53                                                                                                                                                           53 
 54 An example of an empty type is `enum Empty { }`. So, the following will work:                                                                             54 An example of an empty type is `enum Empty { }`. So, the following will work:
 55                                                                                                                                                           55 
 56 ```                                                                                                                                                       56 ```
 57 enum Empty {}                                                                                                                                             57 enum Empty {}
 58                                                                                                                                                           58 
 59 fn foo(x: Empty) {                                                                                                                                        59 fn foo(x: Empty) {
 60     match x {                                                                                                                                             60     match x {
 61         // empty                                                                                                                                          61         // empty
 62     }                                                                                                                                                     62     }
 63 }                                                                                                                                                         63 }
 64 ```                                                                                                                                                       64 ```
 65                                                                                                                                                           65 
 66 However, this won't:                                                                                                                                      66 However, this won't:
 67                                                                                                                                                           67 
 68 ```compile_fail                                                                                                                                           68 ```compile_fail
 69 fn foo(x: Option<String>) {                                                                                                                               69 fn foo(x: Option<String>) {
 70     match x {                                                                                                                                             70     match x {
 71         // empty                                                                                                                                          71         // empty
 72     }                                                                                                                                                     72     }
 73 }                                                                                                                                                         73 }
 74 ```                                                                                                                                                       74 ```
 75 "##,                                                                                                                                                      75 "##,
 76                                                                                                                                                           76 
 77 E0003: r##"                                                                                                                                               77 E0003: r##"
 78 ## Note: this error code is no longer emitted by the compiler.                                                                                            78 #### Note: this error code is no longer emitted by the compiler.
 79                                                                                                                                                           79 
 80 Not-a-Number (NaN) values cannot be compared for equality and hence can never                                                                             80 Not-a-Number (NaN) values cannot be compared for equality and hence can never
 81 match the input to a match expression. So, the following will not compile:                                                                                81 match the input to a match expression. So, the following will not compile:
 82                                                                                                                                                           82 
 83 ```compile_fail                                                                                                                                           83 ```compile_fail
 84 const NAN: f32 = 0.0 / 0.0;                                                                                                                               84 const NAN: f32 = 0.0 / 0.0;
 85                                                                                                                                                           85 
 86 let number = 0.1f32;                                                                                                                                      86 let number = 0.1f32;
 87                                                                                                                                                           87 
 88 match number {                                                                                                                                            88 match number {
 89     NAN => { /* ... */ },                                                                                                                                 89     NAN => { /* ... */ },
 90     _ => {}                                                                                                                                               90     _ => {}
 91 }                                                                                                                                                         91 }
 92 ```                                                                                                                                                       92 ```
 93                                                                                                                                                           93 
 94 To match against NaN values, you should instead use the `is_nan()` method in a                                                                            94 To match against NaN values, you should instead use the `is_nan()` method in a
 95 guard, like so:                                                                                                                                           95 guard, like so:
 96                                                                                                                                                           96 
 97 ```                                                                                                                                                       97 ```
 98 let number = 0.1f32;                                                                                                                                      98 let number = 0.1f32;
 99                                                                                                                                                           99 
100 match number {                                                                                                                                           100 match number {
101     x if x.is_nan() => { /* ... */ }                                                                                                                     101     x if x.is_nan() => { /* ... */ }
102     _ => {}                                                                                                                                              102     _ => {}
103 }                                                                                                                                                        103 }
104 ```                                                                                                                                                      104 ```
105 "##,                                                                                                                                                     105 "##,

