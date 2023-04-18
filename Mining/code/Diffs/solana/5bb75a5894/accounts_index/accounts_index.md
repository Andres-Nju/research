File_Code/solana/5bb75a5894/accounts_index/accounts_index_after.rs --- 1/2 --- Rust
63         !self.is_root(fork) && fork < self.last_root                                                                                                      63         fork < self.last_root

File_Code/solana/5bb75a5894/accounts_index/accounts_index_after.rs --- 2/2 --- Rust
                                                                                                                                                            155         index.add_root(2);
                                                                                                                                                            156         assert!(index.is_purged(1));

