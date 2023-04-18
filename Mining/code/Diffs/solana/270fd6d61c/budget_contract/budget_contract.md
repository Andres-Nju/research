File_Code/solana/270fd6d61c/budget_contract/budget_contract_after.rs --- 1/2 --- Rust
442         let date_is_08601 = "2016-07-08T09:10:11Z";                                                                                                      442         let date_iso8601 = "2016-07-08T09:10:11Z";

File_Code/solana/270fd6d61c/budget_contract/budget_contract_after.rs --- 2/2 --- Rust
481         expected_userdata.extend(date_is_08601.as_bytes());                                                                                              481         expected_userdata.extend(date_iso8601.as_bytes());
482         assert_eq!(tx.userdata, expected_userdata,);                                                                                                     482         assert_eq!(tx.userdata, expected_userdata);

