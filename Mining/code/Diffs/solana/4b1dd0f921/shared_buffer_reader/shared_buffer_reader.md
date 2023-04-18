File_Code/solana/4b1dd0f921/shared_buffer_reader/shared_buffer_reader_after.rs --- Rust
  .                                                                                                                                                          847                                 // Avoid to create more than the number of threads available in the
  .                                                                                                                                                          848                                 // current rayon threadpool. Deadlock could happen otherwise.
847                                 let threads = 8;                                                                                                         849                                 let threads = std::cmp::min(8, rayon::current_num_threads());

