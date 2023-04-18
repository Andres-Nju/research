File_Code/rust/1eacb4a6c3/native/native_after.rs --- Rust
42     let mut assertions = if build.config.llvm_assertions {"ON"} else {"OFF"};                                                                             42     let assertions = if build.config.llvm_assertions {"ON"} else {"OFF"};
43                                                                                                                                                              
44     // Disable LLVM assertions on ARM compilers until #32360 is fixed                                                                                        
45     if target.contains("arm") && target.contains("gnu") {                                                                                                    
46         assertions = "OFF";                                                                                                                                  
47     }                                                                                                                                                        

