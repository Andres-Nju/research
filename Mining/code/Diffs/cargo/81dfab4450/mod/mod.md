File_Code/cargo/81dfab4450/mod/mod_after.rs --- Rust
611     fn tap<F: FnOnce(&mut Self)>(mut self, callback: F) -> Self;                                                                                         611     fn tap<F: FnOnce(&mut Self)>(self, callback: F) -> Self;

