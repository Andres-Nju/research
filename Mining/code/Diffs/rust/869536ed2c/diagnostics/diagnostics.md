File_Code/rust/869536ed2c/diagnostics/diagnostics_after.rs --- Rust
2714 E0225: r##"                                                                                                                                             2714 E0225: r##"
2715 You attempted to use multiple types as bounds for a closure or trait object.                                                                            2715 You attempted to use multiple types as bounds for a closure or trait object.
2716 Rust does not currently support this. A simple example that causes this error:                                                                          2716 Rust does not currently support this. A simple example that causes this error:
2717                                                                                                                                                         2717 
2718 ```compile_fail                                                                                                                                         2718 ```compile_fail
2719 fn main() {                                                                                                                                             2719 fn main() {
2720     let _: Box<std::io::Read+std::io::Write>;                                                                                                           2720     let _: Box<std::io::Read + std::io::Write>;
2721 }                                                                                                                                                       2721 }
2722 ```                                                                                                                                                     2722 ```
2723                                                                                                                                                         2723 
2724 Builtin traits are an exception to this rule: it's possible to have bounds of                                                                           2724 Builtin traits are an exception to this rule: it's possible to have bounds of
2725 one non-builtin type, plus any number of builtin types. For example, the                                                                                2725 one non-builtin type, plus any number of builtin types. For example, the
2726 following compiles correctly:                                                                                                                           2726 following compiles correctly:
2727                                                                                                                                                         2727 
2728 ```                                                                                                                                                     2728 ```
2729 fn main() {                                                                                                                                             2729 fn main() {
2730     let _: Box<std::io::Read+Copy+Sync>;                                                                                                                2730     let _: Box<std::io::Read + Send + Sync>;
2731 }                                                                                                                                                       2731 }
2732 ```                                                                                                                                                     2732 ```
2733 "##,                                                                                                                                                    2733 "##,

