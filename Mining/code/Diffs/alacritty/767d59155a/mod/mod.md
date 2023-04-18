File_Code/alacritty/767d59155a/mod/mod_after.rs --- Rust
 .                                                                                                                                                           91         match self {
 .                                                                                                                                                           92             Error::ShaderCreation(err) => {
91         write!(f, "There was an error initializing the shaders: {}", self)                                                                                93                 write!(f, "There was an error initializing the shaders: {}", err)
                                                                                                                                                             94             },
                                                                                                                                                             95         }

