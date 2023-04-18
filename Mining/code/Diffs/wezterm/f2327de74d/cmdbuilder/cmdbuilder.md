File_Code/wezterm/f2327de74d/cmdbuilder/cmdbuilder_after.rs --- Rust
                                                                                                                                                           137             let home = Self::get_home_dir()?;
                                                                                                                                                           138             let dir: &OsStr = self
                                                                                                                                                           139                 .cwd
                                                                                                                                                           140                 .as_ref()
                                                                                                                                                           141                 .map(|dir| dir.as_os_str())
                                                                                                                                                           142                 .filter(|dir| std::path::Path::new(dir).is_dir())
                                                                                                                                                           143                 .unwrap_or(home.as_ref());
                                                                                                                                                           144             cmd.current_dir(dir);

