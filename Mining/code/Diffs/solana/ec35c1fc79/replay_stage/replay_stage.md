File_Code/solana/ec35c1fc79/replay_stage/replay_stage_after.rs --- 1/3 --- Rust
92         let (mut num_ticks_to_next_vote, slot_height) = {                                                                                                 92         let (mut num_ticks_to_next_vote, slot_height, leader_id) = {

File_Code/solana/ec35c1fc79/replay_stage/replay_stage_after.rs --- 2/3 --- Rust
                                                                                                                                                             97                 rl.get_leader_for_slot(slot).expect("Leader not known"),

File_Code/solana/ec35c1fc79/replay_stage/replay_stage_after.rs --- 3/3 --- Rust
118                 slot_leader: bank.slot_leader(),                                                                                                         119                 slot_leader: leader_id,

