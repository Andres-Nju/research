File_Code/cargo/e38bdca252/network/network_after.rs --- 1/2 --- Rust
5 /// Retry counts provided by Config object 'net.retry'. Config shell outputs                                                                               5 /// Retry counts provided by Config object `net.retry`. Config shell outputs

File_Code/cargo/e38bdca252/network/network_after.rs --- 2/2 --- Rust
10 /// Example:                                                                                                                                              10 /// # Examples
..                                                                                                                                                           11 ///
..                                                                                                                                                           12 /// ```ignore
11 /// use util::network;                                                                                                                                    13 /// use util::network;
12 /// cargo_result = network.with_retry(&config, || something.download());                                                                                  14 /// cargo_result = network.with_retry(&config, || something.download());
                                                                                                                                                             15 /// ```

