File_Code/rust/8f19d5c3f6/iter-step-overflow-ndebug/iter-step-overflow-ndebug_after.rs --- Rust
 .                                                                                                                                                           15     assert_eq!(it.next().unwrap(), 255);
15     assert_eq!(it.next().unwrap(), u8::min_value());                                                                                                      16     assert_eq!(it.next().unwrap(), u8::min_value());
16                                                                                                                                                           17 
17     let mut it = i8::max_value()..;                                                                                                                       18     let mut it = i8::max_value()..;
                                                                                                                                                             19     assert_eq!(it.next().unwrap(), 127);

