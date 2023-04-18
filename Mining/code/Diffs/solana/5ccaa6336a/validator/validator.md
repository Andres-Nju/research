File_Code/solana/5ccaa6336a/validator/validator_after.rs --- Rust
1386     if std::fs::remove_dir_all(account_path).is_err() {                                                                                                 1386     if let Err(e) = std::fs::remove_dir_all(account_path) {
1387         warn!(                                                                                                                                          1387         warn!(
1388             "encountered error removing accounts path: {:?}",                                                                                           1388             "encountered error removing accounts path: {:?}: {}",
1389             account_path                                                                                                                                1389             account_path, e

