File_Code/nushell/20be8a4987/help/help_after.rs --- 1/3 --- Rust
30                 "string to find in command usage",                                                                                                        30                 "string to find in command names, usage, and search terms",

File_Code/nushell/20be8a4987/help/help_after.rs --- 2/3 --- Rust
73                 description: "search for string in command usage",                                                                                        73                 description: "search for string in command names, usage and search terms",

File_Code/nushell/20be8a4987/help/help_after.rs --- 3/3 --- Rust
106             let matches_term = if search_terms.is_empty() {                                                                                              106             let matches_term = if !search_terms.is_empty() {

