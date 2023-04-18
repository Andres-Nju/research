File_Code/solana/54b407b4ca/replicator/replicator_after.rs --- 1/3 --- Rust
86         self.t_window.join().unwrap();                                                                                                                     . 
87         self.fetch_stage.join().unwrap();                                                                                                                 86         self.fetch_stage.join().unwrap();
                                                                                                                                                             87         self.t_window.join().unwrap();

File_Code/solana/54b407b4ca/replicator/replicator_after.rs --- 2/3 --- Rust
                                                                                                                                                            101     use std::fs::remove_dir_all;

File_Code/solana/54b407b4ca/replicator/replicator_after.rs --- 3/3 --- Rust
                                                                                                                                                            181         let _ignored = remove_dir_all(&leader_ledger_path);
                                                                                                                                                            182         let _ignored = remove_dir_all(&replicator_ledger_path);

