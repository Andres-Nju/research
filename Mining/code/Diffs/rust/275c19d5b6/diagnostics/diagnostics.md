File_Code/rust/275c19d5b6/diagnostics/diagnostics_after.rs --- Rust
18 E0001: r##"                                                                                                                                               18 E0001: r##"
19 ## Note: this error code is no longer emitted by the compiler.                                                                                            19 ## Note: this error code is no longer emitted by the compiler.
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
30 ```compile_fail,E0001                                                                                                                                     30 ```
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

