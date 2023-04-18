File_Code/solana/278c125d99/lib/lib_after.rs --- 1/2 --- Rust
298         for (i, instruction_account) in instruction.accounts.iter().enumerate() {                                                                        298         for (i, account_pubkey) in message.account_keys.iter().enumerate() {
299             if !instruction_account.is_writable {                                                                                                        299             if !message.is_writable(i, true) {

File_Code/solana/278c125d99/lib/lib_after.rs --- 2/2 --- Rust
304                 if *account_info.unsigned_key() == instruction_account.pubkey {                                                                          304                 if account_info.unsigned_key() == account_pubkey {

