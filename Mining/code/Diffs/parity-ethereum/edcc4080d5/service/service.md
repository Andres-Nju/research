File_Code/parity-ethereum/edcc4080d5/service/service_after.rs --- 1/3 --- Rust
83         service.init_restore(manifest.clone()).unwrap();                                                                                                  83         service.init_restore(manifest.clone(), true).unwrap();
84         assert!(service.init_restore(manifest.clone()).is_ok());                                                                                          84         assert!(service.init_restore(manifest.clone(), true).is_ok());

File_Code/parity-ethereum/edcc4080d5/service/service_after.rs --- 2/3 --- Rust
132         service.init_restore(manifest.clone()).unwrap();                                                                                                 132         service.init_restore(manifest.clone(), true).unwrap();

File_Code/parity-ethereum/edcc4080d5/service/service_after.rs --- 3/3 --- Rust
138         service.init_restore(manifest.clone()).unwrap();                                                                                                 138         service.init_restore(manifest.clone(), true).unwrap();

