File_Code/solana/b106d3ba60/bank_forks_utils/bank_forks_utils_after.rs --- Rust
73             let snapshot_hash = (deserialized_bank.slot(), deserialized_bank.hash());                                                                     73             let snapshot_hash = (
                                                                                                                                                             74                 deserialized_bank.slot(),
                                                                                                                                                             75                 deserialized_bank.get_accounts_hash(),

