File_Code/parity-ethereum/57d1f2b4d3/lib/lib_after.rs --- 1/2 --- Rust
  .                                                                                                                                                          189         // Mio's behaviour is too unstable for this test. Sometimes we have to wait a few milliseconds,
  .                                                                                                                                                          190         // sometimes more than 5 seconds for the message to arrive.
  .                                                                                                                                                          191         // Therefore we ignore this test in order to not have spurious failure when running continuous
  .                                                                                                                                                          192         // integration.
189         #[test]                                                                                                                                          193         #[test]
                                                                                                                                                             194         #[cfg_attr(feature = "mio", ignore)]

File_Code/parity-ethereum/57d1f2b4d3/lib/lib_after.rs --- 2/2 --- Rust
212                 thread::sleep(Duration::from_secs(5));                                                                                                   217                 thread::sleep(Duration::from_secs(1));

