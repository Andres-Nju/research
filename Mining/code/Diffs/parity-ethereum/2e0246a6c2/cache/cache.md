File_Code/parity-ethereum/2e0246a6c2/cache/cache_after.rs --- 1/2 --- Rust
  .                                                                                                                                                          182                 let duration = Duration::from_secs(20);
182                 let mut cache = Cache::new(Default::default(), Duration::from_secs(5 * 3600));                                                           183                 let mut cache = Cache::new(Default::default(), duration.clone());

File_Code/parity-ethereum/2e0246a6c2/cache/cache_after.rs --- 2/2 --- Rust
189                         *corpus_time = *corpus_time - Duration::from_secs(5 * 3600);                                                                     190                         *corpus_time = *corpus_time - duration;

