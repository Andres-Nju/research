File_Code/parity-ethereum/6e2e08628a/mod/mod_after.rs --- Rust
66         let result = txq.import(TestClient::new(), vec![tx1, tx2].local());                                                                               66         let r1= txq.import(TestClient::new(), vec![tx1].local());
..                                                                                                                                                           67         let r2= txq.import(TestClient::new(), vec![tx2].local());
..                                                                                                                                                           68         assert_eq!(r1, vec![Ok(())]);
67         assert_eq!(result, vec![Ok(()), Err(transaction::Error::LimitReached)]);                                                                          69         assert_eq!(r2, vec![Err(transaction::Error::LimitReached)]);

