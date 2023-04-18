File_Code/tauri/28e4845a89/command/command_after.rs --- Rust
 .                                                                                                                                                           53     let name = command.name;
53     let arg = command.key;                                                                                                                                54     let arg = command.key;
54     Self::deserialize(command).map_err(|e| crate::Error::InvalidArgs(arg, e).into())                                                                      55     Self::deserialize(command).map_err(|e| crate::Error::InvalidArgs(name, arg, e).into())

