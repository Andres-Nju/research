File_Code/rust/9452a8dfa3/metadata/metadata_after.rs --- Rust
1167         || llvm_util::get_major_version() < 7                                                                                                              . 
1168         // LLVM version 7 did not release with an important bug fix;                                                                                    1167         // LLVM version 7 did not release with an important bug fix;
1169         // but the required patch is in the equivalent Rust LLVM.                                                                                       1168         // but the required patch is in the LLVM 8.  Rust LLVM reports
1170         // See https://github.com/rust-lang/rust/issues/57762.                                                                                          1169         // 8 as well.
1171         || (llvm_util::get_major_version() == 7 && unsafe { !llvm::LLVMRustIsRustLLVM() });                                                             1170         || llvm_util::get_major_version() < 8;

