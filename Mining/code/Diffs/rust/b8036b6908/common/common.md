File_Code/rust/b8036b6908/common/common_after.rs --- Rust
  .                                                                                                                                                          237             let words = [u as u64, u.wrapping_shr(64) as u64];
237             llvm::LLVMConstIntOfArbitraryPrecision(t.to_ref(), 2, &u as *const u128 as *const u64)                                                       238             llvm::LLVMConstIntOfArbitraryPrecision(t.to_ref(), 2, words.as_ptr())

