File_Code/solana/07f4789257/in_mem_accounts_index/in_mem_accounts_index_after.rs --- Rust
   .                                                                                                                                                         1096         // in order to return accurate and complete duplicates, we must have nothing left remaining to insert
1096         let inserts = self.startup_info.insert.lock().unwrap();                                                                                         1097         assert!(self.startup_info.insert.lock().unwrap().is_empty());
1097         // in order to return accurate and complete duplicates, we must have nothing left remaining to insert                                                
1098         assert!(inserts.is_empty());                                                                                                                         
1099         drop(inserts);                                                                                                                                       

