File_Code/solana/cc4d75a16f/serde_snapshot/serde_snapshot_after.rs --- Rust
531     let accoounts_db_clone = accounts_db.clone();                                                                                                        531     let accounts_db_clone = accounts_db.clone();
532     let handle = Builder::new()                                                                                                                          532     let handle = Builder::new()
533         .name("notify_account_restore_from_snapshot".to_string())                                                                                        533         .name("notify_account_restore_from_snapshot".to_string())
534         .spawn(move || {                                                                                                                                 534         .spawn(move || {
535             accoounts_db_clone.notify_account_restore_from_snapshot();                                                                                   535             accounts_db_clone.notify_account_restore_from_snapshot();

