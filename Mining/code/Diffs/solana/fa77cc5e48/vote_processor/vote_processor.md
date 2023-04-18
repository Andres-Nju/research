File_Code/solana/fa77cc5e48/vote_processor/vote_processor_after.rs --- Rust
1181             Err(InstructionError::ActiveVoteAccountClose),                                                                                              1181             Err(VoteError::ActiveVoteAccountClose.into()),

