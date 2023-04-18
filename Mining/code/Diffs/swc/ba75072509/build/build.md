File_Code/swc/ba75072509/build/build_after.rs --- Rust
6     let strs = include_str!("words.txt").split("\n").collect::<Vec<_>>();                                                                                  6     let strs = include_str!("words.txt").lines().map(|l| l.trim()).collect::<Vec<_>>();

