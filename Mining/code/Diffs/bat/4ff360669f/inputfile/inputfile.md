File_Code/bat/4ff360669f/inputfile/inputfile_after.rs --- Rust
62                 let file = match File::open(filename) {                                                                                                   62                 let file = File::open(filename).map_err(|e| format!("'{}': {}", filename, e))?;
63                     Ok(f) => f,                                                                                                                              
64                     Err(e) => return Err(format!("{}: {}", filename, e).into()),                                                                             
65                 };                                                                                                                                           

