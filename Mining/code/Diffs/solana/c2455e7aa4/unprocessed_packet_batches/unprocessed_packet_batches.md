File_Code/solana/c2455e7aa4/unprocessed_packet_batches/unprocessed_packet_batches_after.rs --- 1/3 --- Rust
455     fn simmple_deserialized_packet() -> DeserializedPacket {                                                                                             455     fn simple_deserialized_packet() -> DeserializedPacket {

File_Code/solana/c2455e7aa4/unprocessed_packet_batches/unprocessed_packet_batches_after.rs --- 2/3 --- Rust
486         let packet = simmple_deserialized_packet();                                                                                                      486         let packet = simple_deserialized_packet();

File_Code/solana/c2455e7aa4/unprocessed_packet_batches/unprocessed_packet_batches_after.rs --- 3/3 --- Rust
532         let packets_iter = std::iter::repeat_with(simmple_deserialized_packet).take(num_packets);                                                        532         let packets_iter = std::iter::repeat_with(simple_deserialized_packet).take(num_packets);

