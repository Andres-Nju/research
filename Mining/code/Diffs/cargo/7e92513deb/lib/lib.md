File_Code/cargo/7e92513deb/lib/lib_after.rs --- 1/2 --- Rust
43 use serde::Deserialize;                                                                                                                                   43 use serde::de::DeserializeOwned;

File_Code/cargo/7e92513deb/lib/lib_after.rs --- 2/2 --- Rust
106 pub fn call_main_without_stdin<'de, Flags: Deserialize<'de>>(                                                                                            106 pub fn call_main_without_stdin<Flags: DeserializeOwned>(

