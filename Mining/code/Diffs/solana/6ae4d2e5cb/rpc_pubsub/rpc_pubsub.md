File_Code/solana/6ae4d2e5cb/rpc_pubsub/rpc_pubsub_after.rs --- 1/3 --- Rust
90         config: RpcTransactionLogsConfig,                                                                                                                 90         config: Option<RpcTransactionLogsConfig>,

File_Code/solana/6ae4d2e5cb/rpc_pubsub/rpc_pubsub_after.rs --- 2/3 --- Rust
272         config: RpcTransactionLogsConfig,                                                                                                                272         config: Option<RpcTransactionLogsConfig>,

File_Code/solana/6ae4d2e5cb/rpc_pubsub/rpc_pubsub_after.rs --- 3/3 --- Rust
309             config.commitment,                                                                                                                           309             config.and_then(|config| config.commitment),

