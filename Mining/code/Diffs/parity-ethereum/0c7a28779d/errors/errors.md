File_Code/parity-ethereum/0c7a28779d/errors/errors_after.rs --- 1/2 --- Rust
192                                 "Transaction fee is too low. There is another transaction with same nonce in the queue. Try increasing the fee or increm 192                                 "Transaction gas price is too low. There is another transaction with same nonce in the queue. Try increasing the gas pri
    enting the nonce.".into()                                                                                                                                    ce or incrementing the nonce.".into()

File_Code/parity-ethereum/0c7a28779d/errors/errors_after.rs --- 2/2 --- Rust
198                                 format!("Transaction fee is too low. It does not satisfy your node's minimal fee (minimal: {}, got: {}). Try increasing  198                                 format!("Transaction gas price is too low. It does not satisfy your node's minimal gas price (minimal: {}, got: {}). Try
    the fee.", minimal, got)                                                                                                                                      increasing the gas price.", minimal, got)

