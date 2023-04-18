File_Code/solana/ddd0ed0af1/in_mem_accounts_index/in_mem_accounts_index_after.rs --- 1/2 --- Rust
105         self.last_age_flushed.store(age, Ordering::Relaxed);                                                                                             105         self.last_age_flushed.store(age, Ordering::Release);

File_Code/solana/ddd0ed0af1/in_mem_accounts_index/in_mem_accounts_index_after.rs --- 2/2 --- Rust
110         self.last_age_flushed.load(Ordering::Relaxed)                                                                                                    110         self.last_age_flushed.load(Ordering::Acquire)

