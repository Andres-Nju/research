File_Code/bat/1fbdef06aa/app/app_after.rs --- Rust
172                     _ => env::var_os("NO_COLOR").is_none() && self.interactive_output,                                                                   172                     Some("auto") => env::var_os("NO_COLOR").is_none() && self.interactive_output,
                                                                                                                                                             173                     _ => unreachable!("other values for --color are not allowed"),

