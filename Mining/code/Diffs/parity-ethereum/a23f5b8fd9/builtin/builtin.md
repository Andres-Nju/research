File_Code/parity-ethereum/a23f5b8fd9/builtin/builtin_after.rs --- 1/2 --- Rust
 .                                                                                                                                                           23 extern crate machine;
23 extern crate ethcore;                                                                                                                                     24 extern crate ethcore;
                                                                                                                                                             25 extern crate ethcore_builtin;

File_Code/parity-ethereum/a23f5b8fd9/builtin/builtin_after.rs --- 2/2 --- Rust
30 use ethcore::builtin::Builtin;                                                                                                                            32 use ethcore_builtin::Builtin;
31 use ethcore::machine::Machine;                                                                                                                            33 use ethereum_types::H160;
32 use ethereum_types::H160;                                                                                                                                 34 use machine::Machine;
33 use ethcore::ethereum::new_byzantium_test_machine;                                                                                                        35 use machine::test_helpers::new_byzantium_test_machine;

