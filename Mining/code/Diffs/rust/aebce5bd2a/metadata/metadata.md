File_Code/rust/aebce5bd2a/metadata/metadata_after.rs --- 1/3 --- Rust
790     let work_dir = path2cstr(&work_dir).as_ptr();                                                                                                        790     let work_dir = path2cstr(&work_dir);
791     let producer = CString::new(producer).unwrap().as_ptr();                                                                                             791     let producer = CString::new(producer).unwrap();

File_Code/rust/aebce5bd2a/metadata/metadata_after.rs --- 2/3 --- Rust
797             debug_context.builder, compile_unit_name, work_dir);                                                                                         797             debug_context.builder, compile_unit_name, work_dir.as_ptr());

File_Code/rust/aebce5bd2a/metadata/metadata_after.rs --- 3/3 --- Rust
803             producer,                                                                                                                                    803             producer.as_ptr(),

