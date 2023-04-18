File_Code/starship/7124e353db/directory/directory_after.rs --- Rust
75         .arg("--path=/usr")                                                                                                                               75         .arg("--path=/etc")
76         .output()?;                                                                                                                                       76         .output()?;
77     let actual = String::from_utf8(output.stdout).unwrap();                                                                                               77     let actual = String::from_utf8(output.stdout).unwrap();
78                                                                                                                                                           78 
79     let expected = format!("in {} ", Color::Cyan.bold().paint("/usr"));                                                                                   79     let expected = format!("in {} ", Color::Cyan.bold().paint("/etc"));

