File_Code/solana/04d23a1597/append_vec/append_vec_after.rs --- 1/3 --- Rust
289         self.current_len.store(0, Ordering::Relaxed);                                                                                                    289         self.current_len.store(0, Ordering::Release);
290     }                                                                                                                                                    290     }
291                                                                                                                                                          291 
292     pub fn len(&self) -> usize {                                                                                                                         292     pub fn len(&self) -> usize {
293         self.current_len.load(Ordering::Relaxed)                                                                                                         293         self.current_len.load(Ordering::Acquire)

File_Code/solana/04d23a1597/append_vec/append_vec_after.rs --- 2/3 --- Rust
363         let aligned_current_len = u64_align!(self.current_len.load(Ordering::Relaxed));                                                                  363         let aligned_current_len = u64_align!(self.current_len.load(Ordering::Acquire));

File_Code/solana/04d23a1597/append_vec/append_vec_after.rs --- 3/3 --- Rust
422         self.current_len.store(*offset, Ordering::Relaxed);                                                                                              422         self.current_len.store(*offset, Ordering::Release);

