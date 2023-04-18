File_Code/coreutils/b39c4d2756/test_date/test_date_after.rs --- Rust
                                                                                                                                                            46 #[test]
                                                                                                                                                            47 fn test_date_rfc_3339_invalid_arg() {
                                                                                                                                                            48     for param in ["--iso-3339", "--rfc-3"] {
                                                                                                                                                            49         new_ucmd!().arg(format!("{param}=foo")).fails();
                                                                                                                                                            50     }
                                                                                                                                                            51 }

