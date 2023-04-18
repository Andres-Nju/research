File_Code/rust/6d78620bd7/lib/lib_after.rs --- Rust
962         self.cxx[target].path()                                                                                                                          962         match self.cxx.get(target) {
                                                                                                                                                             963             Some(p) => p.path(),
                                                                                                                                                             964             None => panic!("\n\ntarget `{}` is not configured as a host,
                                                                                                                                                             965                             only as a target\n\n", target),
                                                                                                                                                             966         }

