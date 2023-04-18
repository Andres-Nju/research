File_Code/solana/b714a4be63/build/build_after.rs --- Rust
86             assert!(Command::new("./do.sh")                                                                                                               86             assert!(Command::new("bash")
87                 .current_dir("rust")                                                                                                                      87                 .current_dir("rust")
88                 .arg("build")                                                                                                                             88                 .args(&["./do.sh", "build", program])
89                 .arg(program)                                                                                                                             .. 
90                 .status()                                                                                                                                 89                 .status()
91                 .expect("Error calling rust/do.sh")                                                                                                       90                 .expect("Error calling do.sh from build.rs")

