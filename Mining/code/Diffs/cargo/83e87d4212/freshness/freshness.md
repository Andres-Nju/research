File_Code/cargo/83e87d4212/freshness/freshness_after.rs --- Rust
525         fs::copy(&src, &dst).expect("Failed to copy foo");                                                                                               525         fs::hard_link(&src, &dst).expect("Failed to link foo");

