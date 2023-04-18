File_Code/nushell/860c2a606d/engine/engine_after.rs --- Rust
235                 vec![LocationType::Command.spanned(Span::unknown())]                                                                                     235                 vec![LocationType::Command.spanned(Span::new(pos, pos))]
236             } else {                                                                                                                                     236             } else {
237                 // TODO this should be able to be mapped to a command                                                                                    237                 // TODO this should be able to be mapped to a command
238                 vec![LocationType::Argument(command, None).spanned(Span::unknown())]                                                                     238                 vec![LocationType::Argument(command, None).spanned(Span::new(pos, pos))]

