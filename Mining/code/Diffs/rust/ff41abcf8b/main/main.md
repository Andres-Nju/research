File_Code/rust/ff41abcf8b/main/main_after.rs --- 1/3 --- Rust
81                 let frag = fragment.trim_left_matches("#").to_owned();                                                                                    81                 let frag = fragment.trim_start_matches("#").to_owned();

File_Code/rust/ff41abcf8b/main/main_after.rs --- 2/3 --- Rust
346             if rest[..pos_equals].trim_left_matches(" ") != "" {                                                                                         346             if rest[..pos_equals].trim_start_matches(" ") != "" {

File_Code/rust/ff41abcf8b/main/main_after.rs --- 3/3 --- Rust
358             if rest[..pos_quote].trim_left_matches(" ") != "" {                                                                                          358             if rest[..pos_quote].trim_start_matches(" ") != "" {

