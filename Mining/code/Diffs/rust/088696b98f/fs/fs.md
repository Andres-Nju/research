File_Code/rust/088696b98f/fs/fs_after.rs --- Rust
2286         for _ in 0..50 {                                                                                                                                2286         for _ in 0..100 {
....                                                                                                                                                         2287             let dir = tmpdir();
2287             let mut dir = tmpdir().join("a");                                                                                                           2288             let mut dir = dir.join("a");
2288             for _ in 0..100 {                                                                                                                           2289             for _ in 0..40 {

