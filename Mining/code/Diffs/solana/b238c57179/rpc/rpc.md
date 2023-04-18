File_Code/solana/b238c57179/rpc/rpc_after.rs --- Rust
176                 Err(_) => RpcSignatureStatus::GenericFailure,                                                                                            176                 Err(err) => {
                                                                                                                                                             177                     trace!("mapping {:?} to GenericFailure", err);
                                                                                                                                                             178                     RpcSignatureStatus::GenericFailure
                                                                                                                                                             179                 }

