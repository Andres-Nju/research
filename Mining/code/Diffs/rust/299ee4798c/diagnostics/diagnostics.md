File_Code/rust/299ee4798c/diagnostics/diagnostics_after.rs --- Rust
1248 E0433: r##"                                                                                                                                             1248 E0433: r##"
1249 An undeclared type or module was used.                                                                                                                  1249 An undeclared type or module was used.
1250                                                                                                                                                         1250 
1251 Erroneous code example:                                                                                                                                 1251 Erroneous code example:
1252                                                                                                                                                         1252 
1253 ```compile_fail,E0433                                                                                                                                   1253 ```compile_fail,E0433
1254 let map = HashMap::new();                                                                                                                               1254 let map = HashMap::new();
1255 // error: failed to resolve. Use of undeclared type or module `HashMap`                                                                                 1255 // error: failed to resolve. Use of undeclared type or module `HashMap`
1256 ```                                                                                                                                                     1256 ```
1257                                                                                                                                                         1257 
1258 Please verify you didn't misspell the type/module's name or that you didn't                                                                             1258 Please verify you didn't misspell the type/module's name or that you didn't
1259 forgot to import it:                                                                                                                                    1259 forget to import it:
1260                                                                                                                                                         1260 
1261                                                                                                                                                         1261 
1262 ```                                                                                                                                                     1262 ```
1263 use std::collections::HashMap; // HashMap has been imported.                                                                                            1263 use std::collections::HashMap; // HashMap has been imported.
1264 let map: HashMap<u32, u32> = HashMap::new(); // So it can be used!                                                                                      1264 let map: HashMap<u32, u32> = HashMap::new(); // So it can be used!
1265 ```                                                                                                                                                     1265 ```
1266 "##,                                                                                                                                                    1266 "##,

