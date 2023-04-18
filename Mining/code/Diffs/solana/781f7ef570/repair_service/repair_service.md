File_Code/solana/781f7ef570/repair_service/repair_service_after.rs --- 1/2 --- Rust
278                 vec![RepairType::Blob(1, 0), RepairType::Blob(2, 0)]                                                                                     278                 vec![RepairType::HighestBlob(0, 0), RepairType::Blob(2, 0)]

File_Code/solana/781f7ef570/repair_service/repair_service_after.rs --- 2/2 --- Rust
351             // We didn't get the last blob for the slot, so ask for the highest blob for that slot                                                       351             // We didn't get the last blob for this slot, so ask for the highest blob for that slot

