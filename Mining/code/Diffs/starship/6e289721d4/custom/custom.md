File_Code/starship/6e289721d4/custom/custom_after.rs --- Rust
109                 "Could not launch command with given shell or STARSHIP_SHELL env variable, retrying with /bin/env sh"                                    109                 "Could not launch command with given shell or STARSHIP_SHELL env variable, retrying with /usr/bin/env sh"
110             );                                                                                                                                           110             );
111                                                                                                                                                          111 
112             Command::new("/bin/env")                                                                                                                     112             Command::new("/usr/bin/env")

