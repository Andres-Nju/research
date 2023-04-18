File_Code/nushell/7a595827f1/help/help_after.rs --- 1/2 --- Rust
110                 //ReturnSuccess::value(dict.into_value())                                                                                                  

File_Code/nushell/7a595827f1/help/help_after.rs --- 2/2 --- Rust
122                     .drain(..)                                                                                                                           121                     subcommand_names.drain(..).partition(|subcommand_name| {
123                     .partition(|subcommand_name| subcommand_name.starts_with(cmd_name));                                                                 122                         subcommand_name.starts_with(&format!("{} ", cmd_name))
                                                                                                                                                             123                     });

