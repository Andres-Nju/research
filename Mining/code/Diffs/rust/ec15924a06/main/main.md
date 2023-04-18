File_Code/rust/ec15924a06/main/main_after.rs --- 1/2 --- Rust
                                                                                                                                                           494           .join(&testpaths.relative_dir)

File_Code/rust/ec15924a06/main/main_after.rs --- 2/2 --- Rust
                                                                                                                                                           526     if let Some(ref rustdoc_path) = config.rustdoc_path {
                                                                                                                                                           527         inputs.push(mtime(&rustdoc_path));
                                                                                                                                                           528         inputs.push(mtime(&rust_src_dir.join("src/etc/htmldocck.py")));
                                                                                                                                                           529     }

