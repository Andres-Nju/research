File_Code/nushell/1f9907d2ff/mod/mod_after.rs --- Rust
517         r#"                                                                                                                                              517         r#"
518             [[a b]; [1 2]] ++ [[4 5]; [10 11]] | to nuon                                                                                                 518             [[a b]; [1 2]] ++ [[c d]; [10 11]] | to nuon
519         "#                                                                                                                                               519         "#
520     ));                                                                                                                                                  520     ));
521     assert_eq!(actual.out, "[{a: 1, b: 2}, {4: 10, 5: 11}]");                                                                                            521     assert_eq!(actual.out, "[{a: 1, b: 2}, {c: 10, d: 11}]");

