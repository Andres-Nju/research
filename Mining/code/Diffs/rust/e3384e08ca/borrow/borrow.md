File_Code/rust/e3384e08ca/borrow/borrow_after.rs --- Rust
55     /// let s = "a"; // &str                                                                                                                              55     /// let s: &str = "a";
56     /// let ss = s.to_owned(); // String                                                                                                                  56     /// let ss: String = s.to_owned();
57     ///                                                                                                                                                   57     ///
58     /// let v = &[1, 2]; // slice                                                                                                                         58     /// let v: &[i32] = &[1, 2];
59     /// let vv = v.to_owned(); // Vec                                                                                                                     59     /// let vv: Vec<i32> = v.to_owned();

