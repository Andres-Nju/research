File_Code/solana/6dbe7e8bee/drone/drone_after.rs --- Rust
138                     Err(Error::new(ErrorKind::Other, "token limit reached"))                                                                             138                     Err(Error::new(
                                                                                                                                                             139                         ErrorKind::Other,
                                                                                                                                                             140                         format!(
                                                                                                                                                             141                             "token limit reached; req: {} current: {} cap: {}",
                                                                                                                                                             142                             lamports, self.request_current, self.request_cap
                                                                                                                                                             143                         ),

