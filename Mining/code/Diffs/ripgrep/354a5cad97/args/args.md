File_Code/ripgrep/354a5cad97/args/args_after.rs --- Rust
  .                                                                                                                                                          695             #[cfg(unix)]
695             "path:fg:magenta".parse().unwrap(),                                                                                                          696             "path:fg:magenta".parse().unwrap(),
                                                                                                                                                             697             #[cfg(windows)]
                                                                                                                                                             698             "path:fg:cyan".parse().unwrap(),

