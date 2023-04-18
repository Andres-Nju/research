File_Code/rust/23bf3300ad/diagnostics/diagnostics_after.rs --- Rust
3988 E0599: r##"                                                                                                                                             3988 E0599: r##"
3989 ```compile_fail,E0599                                                                                                                                   3989 This error occurs when a method is used on a type which doesn't implement it:
3990 struct Mouth;                                                                                                                                           3990 
3991                                                                                                                                                         3991 Erroneous code example:
3992 let x = Mouth;                                                                                                                                          3992 
3993 x.chocolate(); // error: no method named `chocolate` found for type `Mouth`                                                                             3993 ```compile_fail,E0599
3994                //        in the current scope                                                                                                           3994 struct Mouth;
3995 ```                                                                                                                                                     3995 
....                                                                                                                                                         3996 let x = Mouth;
....                                                                                                                                                         3997 x.chocolate(); // error: no method named `chocolate` found for type `Mouth`
....                                                                                                                                                         3998                //        in the current scope
....                                                                                                                                                         3999 ```
3996 "##,                                                                                                                                                    4000 "##,

