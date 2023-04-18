File_Code/bat/65bb4c7ee6/assets/assets_after.rs --- Rust
32         let theme_set =                                                                                                                                   32         let theme_set = ThemeSet::load_from_folder(&theme_dir).chain_err(|| {
..                                                                                                                                                           33             format!(
33             ThemeSet::load_from_folder(&theme_dir).chain_err(|| "Could not load themes from '{}'")?;                                                      34                 "Could not load themes from '{}'",
                                                                                                                                                             35                 theme_dir.to_string_lossy()
                                                                                                                                                             36             )
                                                                                                                                                             37         })?;

