File_Code/solana/01b00bc593/snapshot_utils/snapshot_utils_after.rs --- Rust
2906         let (_blockhash, fee_calculator) = bank2.last_blockhash_with_fee_calculator();                                                                     . 
2907         let fee = fee_calculator.calculate_fee(tx.message());                                                                                           2906         let fee = bank2
                                                                                                                                                             2907             .get_fee_for_message(&bank2.last_blockhash(), tx.message())
                                                                                                                                                             2908             .unwrap();

