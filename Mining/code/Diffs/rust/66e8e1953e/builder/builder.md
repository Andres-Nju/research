File_Code/rust/66e8e1953e/builder/builder_after.rs --- Rust
                                                                                                                                                           499         if self.sess().target.target.arch == "amdgpu" {
                                                                                                                                                           500             // amdgpu/LLVM does something weird and thinks a i64 value is
                                                                                                                                                           501             // split into a v2i32, halving the bitwidth LLVM expects,
                                                                                                                                                           502             // tripping an assertion. So, for now, just disable this
                                                                                                                                                           503             // optimization.
                                                                                                                                                           504             return;
                                                                                                                                                           505         }

