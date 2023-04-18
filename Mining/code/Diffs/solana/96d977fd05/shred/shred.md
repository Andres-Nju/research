File_Code/solana/96d977fd05/shred/shred_after.rs --- 1/2 --- Rust
1831             let expected_fec_set_index = start_index + ((i / max_per_block) * max_per_block) as u32;                                                    1831             let expected_fec_set_index = start_index + (i - i % max_per_block) as u32;

File_Code/solana/96d977fd05/shred/shred_after.rs --- 2/2 --- Rust
1837             while expected_fec_set_index as usize > data_shreds.len() {                                                                                 1837             while expected_fec_set_index as usize - start_index as usize > data_shreds.len() {

