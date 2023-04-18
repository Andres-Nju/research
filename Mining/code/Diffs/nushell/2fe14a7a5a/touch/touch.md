File_Code/nushell/2fe14a7a5a/touch/touch_after.rs --- Rust
127                 if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<usize>().is_err() {                                                       127                 if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<u64>().is_err() {

