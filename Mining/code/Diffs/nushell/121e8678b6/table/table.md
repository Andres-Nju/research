File_Code/nushell/121e8678b6/table/table_after.rs --- Rust
25     } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {                                                                         25     } else if let Some((Width(w), Height(_))) = terminal_size::terminal_size() {
26         (w - 1) as usize                                                                                                                                  26         w as usize
27     } else {                                                                                                                                              27     } else {
28         80usize                                                                                                                                           28         80

