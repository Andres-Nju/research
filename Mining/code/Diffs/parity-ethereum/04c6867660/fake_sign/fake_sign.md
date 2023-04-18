File_Code/parity-ethereum/04c6867660/fake_sign/fake_sign_after.rs --- 1/2 --- Rust
                                                                                                                                                            18 use std::cmp::min;

File_Code/parity-ethereum/04c6867660/fake_sign/fake_sign_after.rs --- 2/2 --- Rust
24         let max_gas = U256::from(50_000_000);                                                                                                             25         let max_gas = U256::from(500_000_000);
25         let gas = match request.gas {                                                                                                                     26         let gas = min(request.gas.unwrap_or(max_gas), max_gas);
26                 Some(gas) => gas,                                                                                                                            
27                 None => max_gas * 10_u32,                                                                                                                    
28         };                                                                                                                                                   

