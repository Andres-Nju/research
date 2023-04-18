File_Code/solana/773b2f23f4/streamer/streamer_after.rs --- Rust
  .                                                                                                                                                          495         assert!(stats.packet_batches_count.load(Ordering::Relaxed) >= 1);
495         assert_eq!(stats.packets_count.load(Ordering::Relaxed), NUM_PACKETS);                                                                            496         assert_eq!(stats.packets_count.load(Ordering::Relaxed), NUM_PACKETS);
496         assert_eq!(stats.packet_batches_count.load(Ordering::Relaxed), 1);                                                                                   

