File_Code/solana/fe1e08b9ad/stake_state/stake_state_after.rs --- 1/3 --- Rust
                                                                                                                                                           496         stake_lamports: u64,

File_Code/solana/fe1e08b9ad/stake_state/stake_state_after.rs --- 2/3 --- Rust
                                                                                                                                                           509         self.delegation.stake = stake_lamports;

File_Code/solana/fe1e08b9ad/stake_state/stake_state_after.rs --- 3/3 --- Rust
                                                                                                                                                           708                     self.lamports()?.saturating_sub(meta.rent_exempt_reserve), // can't stake the rent ;)

