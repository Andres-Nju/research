File_Code/rust-analyzer/d0811c4066/handlers/handlers_after.rs --- 1/2 --- Rust
12 use ra_syntax::{TextUnit, text_utils::contains_offset_nonstrict};                                                                                         12 use ra_syntax::{TextUnit, text_utils::{contains_offset_nonstrict, intersect}};

File_Code/rust-analyzer/d0811c4066/handlers/handlers_after.rs --- 2/2 --- Rust
621         .filter(|(range, _fix)| contains_offset_nonstrict(*range, range.start()))                                                                        621         .filter(|(diag_range, _fix)| intersect(*diag_range, range).is_some())

