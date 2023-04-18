File_Code/parity-ethereum/53add78a28/dapps/dapps_after.rs --- 1/2 --- Rust
98                                 .transaction_proof(ctx, on_demand::request::TransactionProof {                                                            98                                 .request(ctx, on_demand::request::TransactionProof {

File_Code/parity-ethereum/53add78a28/dapps/dapps_after.rs --- 2/2 --- Rust
107                                         header: header,                                                                                                  107                                         header: on_demand::request::HeaderRef::Stored(header),
108                                         env_info: env_info,                                                                                              108                                         env_info: env_info,
109                                         engine: self.client.engine().clone(),                                                                            109                                         engine: self.client.engine().clone(),
110                                 })                                                                                                                       110                                 })
                                                                                                                                                             111                                 .expect("todo: handle error")

