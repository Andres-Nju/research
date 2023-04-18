File_Code/rust/c666bd5570/intrinsic/intrinsic_after.rs --- 1/2 --- Rust
66 pub fn intrisic_operation_unsafety(intrinsic: &str) -> hir::Unsafety {                                                                                    66 pub fn intrinsic_operation_unsafety(intrinsic: &str) -> hir::Unsafety {

File_Code/rust/c666bd5570/intrinsic/intrinsic_after.rs --- 2/2 --- Rust
133         let unsafety = intrisic_operation_unsafety(&name[..]);                                                                                           133         let unsafety = intrinsic_operation_unsafety(&name[..]);

