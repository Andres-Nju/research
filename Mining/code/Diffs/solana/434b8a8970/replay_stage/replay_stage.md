File_Code/solana/434b8a8970/replay_stage/replay_stage_after.rs --- Rust
126                         let vote = VoteTransaction::new_vote(keypair, bank.id(), bank.last_id(), 0);                                                     126                         let vote =
                                                                                                                                                             127                             VoteTransaction::new_vote(keypair, bank.slot(), bank.last_id(), 0);

