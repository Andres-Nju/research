File_Code/rust/bbc3cd4378/lib/lib_after.rs --- Rust
768                 let sha = self.rust_info.sha().expect("failed to find sha");                                                                             768                 let sha = self.rust_sha().unwrap_or(channel::CFG_RELEASE_NUM);

