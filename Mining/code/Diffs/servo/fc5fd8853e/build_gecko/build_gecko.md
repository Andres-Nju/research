File_Code/servo/fc5fd8853e/build_gecko/build_gecko_after.rs --- 1/2 --- Rust
553         let script = Path::new(file!()).parent().unwrap().join("gecko").join("regen_atoms.py");                                                          553         let script = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())

File_Code/servo/fc5fd8853e/build_gecko/build_gecko_after.rs --- 2/2 --- Rust
...                                                                                                                                                          600     use std::env;
599     use std::path::Path;                                                                                                                                 601     use std::path::PathBuf;
600     use super::common::*;                                                                                                                                602     use super::common::*;
601                                                                                                                                                          603 
602     pub fn generate() {                                                                                                                                  604     pub fn generate() {
603         let dir = Path::new(file!()).parent().unwrap().join("gecko/generated");                                                                          605         let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("gecko/generated");

