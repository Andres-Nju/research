File_Code/solana/369d376d10/rpc_subscriptions/rpc_subscriptions_after.rs --- 1/2 --- Rust
637             .unwrap_or((None, Some(false)));                                                                                                             637             .unwrap_or_default();

File_Code/solana/369d376d10/rpc_subscriptions/rpc_subscriptions_after.rs --- 2/2 --- Rust
973                     if is_received_notification_enabled                                                                                                  973                     if is_received_notification_enabled.unwrap_or_default() {
974                         .expect("All signature subscriptions must have this config field set")                                                               

