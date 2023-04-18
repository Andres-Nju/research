File_Code/bat/19aa878a08/main/main_after.rs --- 1/2 --- Rust
68     let mut map: HashMap<&str, Vec<String>> = HashMap::new();                                                                                             68     let mut map = HashMap::new();

File_Code/bat/19aa878a08/main/main_after.rs --- 2/2 --- Rust
73                 let globs = map.entry(s).or_insert_with(Vec::new);                                                                                        73                 let globs = map.entry(*s).or_insert_with(Vec::new);

