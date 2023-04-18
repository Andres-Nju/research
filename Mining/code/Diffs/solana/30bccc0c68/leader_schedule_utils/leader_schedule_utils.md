File_Code/solana/30bccc0c68/leader_schedule_utils/leader_schedule_utils_after.rs --- Rust
39     let epoch = slot / bank.slots_per_epoch();                                                                                                             . 
40     slot_leader_by(bank, |_, _, _| (slot, epoch))                                                                                                         39     slot_leader_by(bank, |_, _, _| {
                                                                                                                                                             40         (slot % bank.slots_per_epoch(), slot / bank.slots_per_epoch())
                                                                                                                                                             41     })

