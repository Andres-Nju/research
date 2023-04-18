File_Code/solana/9cb0151544/rpc_subscriptions/rpc_subscriptions_after.rs --- Rust
                                                                                                                                                          1414             // Sleep here to ensure adequate time for the async thread to fully process the
                                                                                                                                                          1415             // subscribed notification before the bank transaction is processed. Without this
                                                                                                                                                          1416             // sleep, the bank transaction ocassionally completes first and we hang forever
                                                                                                                                                          1417             // waiting to receive a bank notification.
                                                                                                                                                          1418             std::thread::sleep(Duration::from_millis(100));

