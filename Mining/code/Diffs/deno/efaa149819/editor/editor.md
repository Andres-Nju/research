File_Code/deno/efaa149819/editor/editor_after.rs --- Rust
  .                                                                                                                                                          426       if cfg!(target_os = "windows") {
  .                                                                                                                                                          427         // Inserting a tab is broken in windows with rustyline
  .                                                                                                                                                          428         // use 4 spaces as a workaround for now
  .                                                                                                                                                          429         Some(Cmd::Insert(n, "    ".into()))
425     {                                                                                                                                                    430       } else {
426       Some(Cmd::Insert(n, "\t".into()))                                                                                                                  431         Some(Cmd::Insert(n, "\t".into()))
427     } else {                                                                                                                                             432       }

