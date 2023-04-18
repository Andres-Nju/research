File_Code/solana/02b26ddd82/sigverify_stage/sigverify_stage_after.rs --- 1/3 --- Rust
331         let mut num_valid_packets = num_unique;                                                                                                          331         let mut num_packets_to_verify = num_unique;

File_Code/solana/02b26ddd82/sigverify_stage/sigverify_stage_after.rs --- 2/3 --- Rust
339             num_valid_packets = MAX_SIGVERIFY_BATCH;                                                                                                     339             num_packets_to_verify = MAX_SIGVERIFY_BATCH;

File_Code/solana/02b26ddd82/sigverify_stage/sigverify_stage_after.rs --- 3/3 --- Rust
348         let mut batches = verifier.verify_batches(batches, num_valid_packets);                                                                           348         let mut batches = verifier.verify_batches(batches, num_packets_to_verify);
349         count_valid_packets(                                                                                                                             349         let num_valid_packets = count_valid_packets(

