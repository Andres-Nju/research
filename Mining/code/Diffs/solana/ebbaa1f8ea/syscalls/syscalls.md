File_Code/solana/ebbaa1f8ea/syscalls/syscalls_after.rs --- 1/2 --- Rust
351         Ok(unsafe { from_raw_parts_mut(0x1 as *mut T, len as usize) })                                                                                   351         Ok(&mut [])

File_Code/solana/ebbaa1f8ea/syscalls/syscalls_after.rs --- 2/2 --- Rust
1474     let size = num_accounts * size_of::<AccountMeta>() + data_len;                                                                                      1474     let size = num_accounts
                                                                                                                                                             1475         .saturating_mul(size_of::<AccountMeta>())
                                                                                                                                                             1476         .saturating_add(data_len);

