File_Code/alacritty/e0f8320c39/cli/cli_after.rs --- 1/4 --- Rust
15 #[derive(StructOpt, Debug)]                                                                                                                               15 #[derive(StructOpt, Default, Debug)]

File_Code/alacritty/e0f8320c39/cli/cli_after.rs --- 2/4 --- Rust
286         Options::new().override_config(&mut config);                                                                                                     286         Options::default().override_config(&mut config);

File_Code/alacritty/e0f8320c39/cli/cli_after.rs --- 3/4 --- Rust
295         let options = Options { title: Some("foo".to_owned()), ..Options::new() };                                                                       295         let options = Options { title: Some("foo".to_owned()), ..Options::default() };

File_Code/alacritty/e0f8320c39/cli/cli_after.rs --- 4/4 --- Rust
306         Options::new().override_config(&mut config);                                                                                                     306         Options::default().override_config(&mut config);

