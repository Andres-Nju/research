File_Code/solana/38cd29810f/accounts_index/accounts_index_after.rs --- 1/2 --- Rust
273         self.ref_count.load(Ordering::Relaxed)                                                                                                           273         self.ref_count.load(Ordering::Acquire)

File_Code/solana/38cd29810f/accounts_index/accounts_index_after.rs --- 2/2 --- Rust
278             self.ref_count.fetch_add(1, Ordering::Relaxed);                                                                                              278             self.ref_count.fetch_add(1, Ordering::Release);
279         } else {                                                                                                                                         279         } else {
280             self.ref_count.fetch_sub(1, Ordering::Relaxed);                                                                                              280             self.ref_count.fetch_sub(1, Ordering::Release);

