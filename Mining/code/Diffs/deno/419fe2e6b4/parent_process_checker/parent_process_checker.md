File_Code/deno/419fe2e6b4/parent_process_checker/parent_process_checker_after.rs --- Rust
65     assert_eq!(is_process_active(pid), true);                                                                                                             65     assert!(is_process_active(pid));
66     child.kill().unwrap();                                                                                                                                66     child.kill().unwrap();
67     child.wait().unwrap();                                                                                                                                67     child.wait().unwrap();
68     assert_eq!(is_process_active(pid), false);                                                                                                            68     assert!(!is_process_active(pid));

