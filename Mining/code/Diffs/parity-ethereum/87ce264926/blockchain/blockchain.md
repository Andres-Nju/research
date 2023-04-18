File_Code/parity-ethereum/87ce264926/blockchain/blockchain_after.rs --- 1/3 --- Rust
464         out.write_fmt(format_args!("{{ \"state\": [", )).expect("Couldn't write to stream.");                                                            464         out.write_fmt(format_args!("{{ \"state\": {{", )).expect("Couldn't write to stream.");

File_Code/parity-ethereum/87ce264926/blockchain/blockchain_after.rs --- 2/3 --- Rust
501                                                 let mut si = 0;                                                                                          ... 
502                                                 for key in keys.into_iter() {                                                                            501                                                 for key in keys.into_iter() {
503                                                         if si != 0 {                                                                                     502                                                         if last_storage.is_some() {
504                                                                 out.write(b",").expect("Write error");                                                   503                                                                 out.write(b",").expect("Write error");
505                                                         }                                                                                                504                                                         }
506                                                         out.write_fmt(format_args!("\n\t\"0x{}\": \"0x{}\"", key.hex(), client.storage_at(&account, &key 505                                                         out.write_fmt(format_args!("\n\t\"0x{}\": \"0x{}\"", key.hex(), client.storage_at(&account, &key
... , at).unwrap_or_else(Default::default).hex())).expect("Write error");                                                                                        , at).unwrap_or_else(Default::default).hex())).expect("Write error");
507                                                         si += 1;                                                                                             

File_Code/parity-ethereum/87ce264926/blockchain/blockchain_after.rs --- 3/3 --- Rust
522         out.write_fmt(format_args!("\n]}}")).expect("Write error");                                                                                      520         out.write_fmt(format_args!("\n}}}}")).expect("Write error");

