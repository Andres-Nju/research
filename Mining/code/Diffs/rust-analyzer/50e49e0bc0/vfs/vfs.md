File_Code/rust-analyzer/50e49e0bc0/vfs/vfs_after.rs --- Rust
  .                                                                                                                                                          121     // NOTE: Windows generates extra `Write` events when renaming?
  .                                                                                                                                                          122     // meaning we have extra tasks to process
121     process_tasks(&mut vfs, 2);                                                                                                                          123     process_tasks(&mut vfs, if cfg!(windows) { 4 } else { 2 });

