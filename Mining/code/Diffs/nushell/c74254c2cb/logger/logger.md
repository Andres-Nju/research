File_Code/nushell/c74254c2cb/logger/logger_after.rs --- Rust
100     if !matches!(log_target, LogTarget::File) {                                                                                                          100     if matches!(
...                                                                                                                                                          101         log_target,
...                                                                                                                                                          102         LogTarget::Stdout | LogTarget::Stderr | LogTarget::Mixed
...                                                                                                                                                          103     ) {
101         set_colored_level(builder, level);                                                                                                               104         Level::iter().for_each(|level| set_colored_level(builder, level));

