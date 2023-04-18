File_Code/solana/56667e17c9/rpc_client/rpc_client_after.rs --- 1/2 --- Rust
127             serde_json::from_value(signature_status).unwrap();                                                                                           127             serde_json::from_value(signature_status)
                                                                                                                                                             128                 .map_err(|err| ClientError::new_with_command(err.into(), "GetSignatureStatus"))?;

File_Code/solana/56667e17c9/rpc_client/rpc_client_after.rs --- 2/2 --- Rust
957             serde_json::from_value(response).unwrap();                                                                                                   958             .map_err(|err| ClientError::new_with_command(err.into(), "GetSignatureStatus"))?;

