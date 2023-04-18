File_Code/rust/8d7fb87e65/fs/fs_after.rs --- Rust
3319             symlink_dir("../d/e", &c).unwrap();                                                                                                         3319             symlink_file("../d/e", &c).unwrap();
3320             symlink_file("../f", &e).unwrap();                                                                                                          3320             symlink_file("../f", &e).unwrap();
3321         }                                                                                                                                               3321         }
3322         if cfg!(windows) {                                                                                                                              3322         if cfg!(windows) {
3323             symlink_dir(r"..\d\e", &c).unwrap();                                                                                                        3323             symlink_file(r"..\d\e", &c).unwrap();

