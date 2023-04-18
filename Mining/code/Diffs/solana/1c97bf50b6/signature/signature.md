File_Code/solana/1c97bf50b6/signature/signature_after.rs --- Rust
134         let mut seed = [0u8; 32];                                                                                                                          . 
135         seed[0..3].copy_from_slice(&[1, 2, 3, 4]);                                                                                                         . 
136         let rnd = GenKeys::new(seed);                                                                                                                    134         let rnd = GenKeys::new([0u8; 32]);

