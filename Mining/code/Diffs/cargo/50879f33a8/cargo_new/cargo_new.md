File_Code/cargo/50879f33a8/cargo_new/cargo_new_after.rs --- Rust
406     let in_existing_vcs_repo = existing_vcs_repo(path.parent().unwrap(), config.cwd());                                                                  406     let in_existing_vcs_repo = existing_vcs_repo(path.parent().unwrap_or(path), config.cwd());

