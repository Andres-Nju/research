File_Code/rust/18ff6410d8/slice/slice_after.rs --- Rust
1499 /// satisfied, for every `i` in `0 .. runs.len() - 2`:                                                                                                  1499 /// satisfied:
1500 ///                                                                                                                                                     1500 ///
1501 /// 1. `runs[i].len > runs[i + 1].len`                                                                                                                  1501 /// 1. for every `i` in `1..runs.len()`: `runs[i - 1].len > runs[i].len`
1502 /// 2. `runs[i].len > runs[i + 1].len + runs[i + 2].len`                                                                                                1502 /// 2. for every `i` in `2..runs.len()`: `runs[i - 2].len > runs[i - 1].len + runs[i].len`

