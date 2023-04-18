File_Code/rust/472f4e1cc8/builder/builder_after.rs --- Rust
360                 let lib = if compiler.stage >= 1 && builder.build.config.libdir_relative.is_some() {                                                     360                 let lib = if compiler.stage >= 1 && builder.build.config.libdir.is_some() {
361                     builder.build.config.libdir_relative.clone().unwrap()                                                                                361                     builder.build.config.libdir.clone().unwrap()

