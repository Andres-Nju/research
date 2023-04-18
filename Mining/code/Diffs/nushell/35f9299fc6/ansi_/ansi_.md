File_Code/nushell/35f9299fc6/ansi_/ansi__after.rs --- Rust
384             // OCS's need to end with a bell '\x07' char                                                                                                 384             // OCS's need to end with either:
...                                                                                                                                                          385             // bel '\x07' char
...                                                                                                                                                          386             // string terminator aka st '\\' char
385             format!("\x1b]{};", code_string)                                                                                                             387             format!("\x1b]{}", code_string)

