File_Code/solana/afaf95cf53/validator/validator_after.rs --- Rust
449     let genesis_block =                                                                                                                                  449     let genesis_block = GenesisBlock::load(blocktree_path).expect("Failed to load genesis block");
450         GenesisBlock::load(blocktree_path).expect("Expected to successfully open genesis block");                                                        ... 
451                                                                                                                                                          450 
452     let (blocktree, ledger_signal_receiver, completed_slots_receiver) =                                                                                  451     let (blocktree, ledger_signal_receiver, completed_slots_receiver) =
453         Blocktree::open_with_signal(blocktree_path)                                                                                                      452         Blocktree::open_with_signal(blocktree_path).expect("Failed to open ledger database");
454             .expect("Expected to successfully open database ledger");                                                                                        

