File_Code/nushell/e737222a5d/scope/scope_after.rs --- 1/2 --- Rust
                                                                                                                                                           106             frame_command_names.extend(frame.get_alias_names());
                                                                                                                                                           107             frame_command_names.extend(frame.get_custom_command_names());

File_Code/nushell/e737222a5d/scope/scope_after.rs --- 2/2 --- Rust
                                                                                                                                                           393     pub fn get_custom_command_names(&self) -> Vec<String> {
                                                                                                                                                           394         self.custom_commands.keys().map(|x| x.to_string()).collect()
                                                                                                                                                           395     }

