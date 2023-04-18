File_Code/solana/2f445c70b7/syscalls/syscalls_after.rs --- 1/2 --- Rust
4472                 size_of::<u8>() as u64,                                                                                                                 4472                 size_of::<T>() as u64,

File_Code/solana/2f445c70b7/syscalls/syscalls_after.rs --- 2/2 --- Rust
4483                 (address as *const u8 as usize).wrapping_rem(align_of::<u8>()),                                                                         4483                 (address as *const u8 as usize).wrapping_rem(align_of::<T>()),

