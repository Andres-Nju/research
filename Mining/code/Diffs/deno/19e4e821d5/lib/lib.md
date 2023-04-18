File_Code/deno/19e4e821d5/lib/lib_after.rs --- 1/3 --- Rust
1498       NativeType::I64 => {                                                                                                                              1498       NativeType::I64 | NativeType::ISize => {

File_Code/deno/19e4e821d5/lib/lib_after.rs --- 2/3 --- Rust
1507       NativeType::U64 => {                                                                                                                              1507       NativeType::U64 | NativeType::USize => {

File_Code/deno/19e4e821d5/lib/lib_after.rs --- 3/3 --- Rust
1523       _ => {                                                                                                                                            1523       NativeType::Void => unreachable!(),
1524         unreachable!()                                                                                                                                       
1525       }                                                                                                                                                      

