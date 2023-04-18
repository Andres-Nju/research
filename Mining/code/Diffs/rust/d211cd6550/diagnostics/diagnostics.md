File_Code/rust/d211cd6550/diagnostics/diagnostics_after.rs --- Rust
2024 E0185: r##"                                                                                                                                             2024 E0185: r##"
2025 An associated function for a trait was defined to be static, but an                                                                                     2025 An associated function for a trait was defined to be static, but an
2026 implementation of the trait declared the same function to be a method (i.e. to                                                                          2026 implementation of the trait declared the same function to be a method (i.e. to
2027 take a `self` parameter).                                                                                                                               2027 take a `self` parameter).
2028                                                                                                                                                         2028 
2029 Here's an example of this error:                                                                                                                        2029 Here's an example of this error:
2030                                                                                                                                                         2030 
2031 ```compile_fail                                                                                                                                         2031 ```compile_fail
2032 trait Foo {                                                                                                                                             2032 trait Foo {
2033     fn foo();                                                                                                                                           2033     fn foo();
2034 }                                                                                                                                                       2034 }
2035                                                                                                                                                         2035 
2036 struct Bar;                                                                                                                                             2036 struct Bar;
2037                                                                                                                                                         2037 
2038 impl Foo for Bar {                                                                                                                                      2038 impl Foo for Bar {
2039     // error, method `foo` has a `&self` declaration in the impl, but not in                                                                            2039     // error, method `foo` has a `&self` declaration in the impl, but not in
2040     // the trait                                                                                                                                        2040     // the trait
2041     fn foo(&self) {}                                                                                                                                    2041     fn foo(&self) {}
2042 }                                                                                                                                                       2042 }
....                                                                                                                                                         2043 ```
2043 "##,                                                                                                                                                    2044 "##,

