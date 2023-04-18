File_Code/rust/365eafbc7f/macros/macros_after.rs --- 1/2 --- Rust
493     /// A macro which stringifies its argument.                                                                                                          493     /// A macro which stringifies its arguments.

File_Code/rust/365eafbc7f/macros/macros_after.rs --- 2/2 --- Rust
510     macro_rules! stringify { ($t:tt) => ({ /* compiler built-in */ }) }                                                                                  510     macro_rules! stringify { ($($t:tt)*) => ({ /* compiler built-in */ }) }

