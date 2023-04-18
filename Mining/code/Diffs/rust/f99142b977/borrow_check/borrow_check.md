File_Code/rust/f99142b977/borrow_check/borrow_check_after.rs --- Rust
1172     /// Finds the span of arguments of aclosure (within `maybe_closure_span`) and its usage of                                                          1172     /// Finds the span of arguments of a closure (within `maybe_closure_span`) and its usage of
1173     /// the local assigned at `location`.                                                                                                               1173     /// the local assigned at `location`.
                                                                                                                                                             1174     /// This is done by searching in statements succeeding `location`
                                                                                                                                                             1175     /// and originating from `maybe_closure_span`.

