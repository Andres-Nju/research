File_Code/wasmer/1e4fa78e11/export/export_after.rs --- 1/2 --- Rust
16 use wasmer_runtime::{Instance, Memory, Module, Value};                                                                                                    16 use wasmer_runtime::{Instance, Module, Value};

File_Code/wasmer/1e4fa78e11/export/export_after.rs --- 2/2 --- Rust
 ..                                                                                                                                                          358         let mem = Box::new(exported_memory.clone());
358         *memory = exported_memory as *const Memory as *mut wasmer_memory_t;                                                                              359         *memory = Box::into_raw(mem) as *mut wasmer_memory_t;

