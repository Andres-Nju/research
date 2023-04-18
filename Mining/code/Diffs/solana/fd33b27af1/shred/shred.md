File_Code/solana/fd33b27af1/shred/shred_after.rs --- 1/3 --- Rust
                                                                                                                                                           268     fec_set_shred_start: usize,

File_Code/solana/fd33b27af1/shred/shred_after.rs --- 2/3 --- Rust
427             let data_ptrs: Vec<_> = self                                                                                                                 428             let data_ptrs: Vec<_> = self.shreds[self.fec_set_shred_start..]

File_Code/solana/fd33b27af1/shred/shred_after.rs --- 3/3 --- Rust
                                                                                                                                                             469             self.fec_set_shred_start = self.shreds.len();

