File_Code/servo/7db47f8084/document_loader/document_loader_after.rs --- Rust
133         self.blocking_loads.remove(idx.expect(&format!("unknown completed load {:?}", load)));                                                           133         self.blocking_loads.remove(idx.unwrap_or_else(|| panic!("unknown completed load {:?}", load)));

