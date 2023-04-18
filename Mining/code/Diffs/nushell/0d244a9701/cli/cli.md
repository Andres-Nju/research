File_Code/nushell/0d244a9701/cli/cli_after.rs --- Rust
679                                 _ => {                                                                                                                   679                                 Ok(None) => break,
680                                     break;                                                                                                               680                                 Err(e) => return LineResult::Error(line.to_string(), e),
681                                 }                                                                                                                            

