File_Code/cargo/c036952f28/interning/interning_after.rs --- 1/4 --- Rust
11 pub fn leek(s: String) -> &'static str {                                                                                                                  11 pub fn leak(s: String) -> &'static str {

File_Code/cargo/c036952f28/interning/interning_after.rs --- 2/4 --- Rust
23     static ref STRING_CASHE: RwLock<HashSet<&'static str>> =                                                                                              23     static ref STRING_CACHE: RwLock<HashSet<&'static str>> =

File_Code/cargo/c036952f28/interning/interning_after.rs --- 3/4 --- Rust
35         let mut cache = STRING_CASHE.write().unwrap();                                                                                                    35         let mut cache = STRING_CACHE.write().unwrap();

File_Code/cargo/c036952f28/interning/interning_after.rs --- 4/4 --- Rust
42         let s = leek(str.to_string());                                                                                                                    42         let s = leak(str.to_string());

