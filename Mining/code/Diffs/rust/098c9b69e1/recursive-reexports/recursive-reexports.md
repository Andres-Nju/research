File_Code/rust/098c9b69e1/recursive-reexports/recursive-reexports_after.rs --- Rust
 .                                                                                                                                                           13 extern crate recursive_reexports;
12                                                                                                                                                           14 
13 fn f() -> recursive_reexports::S {} //~ ERROR undeclared                                                                                                  15 fn f() -> recursive_reexports::S {} //~ ERROR type name `recursive_reexports::S` is undefined

