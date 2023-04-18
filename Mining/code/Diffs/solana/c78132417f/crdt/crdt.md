File_Code/solana/c78132417f/crdt/crdt_after.rs --- 1/4 --- Rust
162     timeout: Duration,                                                                                                                                     

File_Code/solana/c78132417f/crdt/crdt_after.rs --- 2/4 --- Rust
187             timeout: Duration::from_millis(100),                                                                                                           

File_Code/solana/c78132417f/crdt/crdt_after.rs --- 3/4 --- Rust
513         let timeout = obj.read().unwrap().timeout.clone();                                                                                                 

File_Code/solana/c78132417f/crdt/crdt_after.rs --- 4/4 --- Rust
521                 //TODO this should be a tuned parameter                                                                                                  518                 //TODO: possibly tune this parameter
...                                                                                                                                                          519                 //we saw a deadlock passing an obj.read().unwrap().timeout into sleep
522                 sleep(timeout);                                                                                                                          520                 sleep(Duration::from_millis(100));

