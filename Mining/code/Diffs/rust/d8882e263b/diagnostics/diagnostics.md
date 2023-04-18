File_Code/rust/d8882e263b/diagnostics/diagnostics_after.rs --- Rust
733 E0061: r##"                                                                                                                                              733 E0061: r##"
734 The number of arguments passed to a function must match the number of arguments                                                                          734 The number of arguments passed to a function must match the number of arguments
735 specified in the function signature.                                                                                                                     735 specified in the function signature.
736                                                                                                                                                          736 
737 For example, a function like:                                                                                                                            737 For example, a function like:
738                                                                                                                                                          738 
739 ```                                                                                                                                                      739 ```
740 fn f(a: u16, b: &str) {}                                                                                                                                 740 fn f(a: u16, b: &str) {}
741 ```                                                                                                                                                      741 ```
742                                                                                                                                                          742 
743 Must always be called with exactly two arguments, e.g. `f(2, "test")`.                                                                                   743 Must always be called with exactly two arguments, e.g. `f(2, "test")`.
744                                                                                                                                                          744 
745 Note, that Rust does not have a notion of optional function arguments or                                                                                 745 Note that Rust does not have a notion of optional function arguments or
746 variadic functions (except for its C-FFI).                                                                                                               746 variadic functions (except for its C-FFI).
747 "##,                                                                                                                                                     747 "##,

