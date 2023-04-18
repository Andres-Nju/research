File_Code/solana/1be7ee51be/banking_stage/banking_stage_after.rs --- Rust
153             Some(bank) => leader_schedule_utils::slot_leader_at(bank.slot() + 1, &bank).unwrap(),                                                        153             Some(bank) => {
                                                                                                                                                             154                 leader_schedule_utils::slot_leader_at(bank.slot() + 1, &bank).unwrap_or_default()
                                                                                                                                                             155             }

