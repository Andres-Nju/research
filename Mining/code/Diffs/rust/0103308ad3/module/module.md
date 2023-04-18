File_Code/rust/0103308ad3/module/module_after.rs --- 1/2 --- Rust
215             relative_prefix_string = format!("{}{}", ident, path::MAIN_SEPARATOR);                                                                       215             relative_prefix_string = format!("{}{}", ident.name, path::MAIN_SEPARATOR);

File_Code/rust/0103308ad3/module/module_after.rs --- 2/2 --- Rust
221         let mod_name = id.to_string();                                                                                                                   221         let mod_name = id.name.to_string();

