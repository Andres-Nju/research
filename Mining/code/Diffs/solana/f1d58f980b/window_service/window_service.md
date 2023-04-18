File_Code/solana/f1d58f980b/window_service/window_service_after.rs --- Rust
  .                                                                                                                                                          109         // Ignore the send error, as the retransmit is optional (e.g. replicators don't retransmit)
109         match retransmit.send(packets) {                                                                                                                 110         let _ = retransmit.send(packets);
110             Ok(_) => Ok(()),                                                                                                                                 
111             Err(e) => Err(e),                                                                                                                                
112         }?;                                                                                                                                                  

