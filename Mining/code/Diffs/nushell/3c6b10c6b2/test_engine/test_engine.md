File_Code/nushell/3c6b10c6b2/test_engine/test_engine_after.rs --- Rust
                                                                                                                                                           369 /// Issue #7872
                                                                                                                                                           370 #[test]
                                                                                                                                                           371 fn assignment_to_in_var_no_panic() -> TestResult {
                                                                                                                                                           372     fail_test(r#"$in = 3"#, "needs to be a mutable variable")
                                                                                                                                                           373 }

