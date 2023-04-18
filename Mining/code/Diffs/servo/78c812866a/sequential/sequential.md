File_Code/servo/78c812866a/sequential/sequential_after.rs --- Rust
27             // NB: Data is unused now, but we can always decrement the count                                                                              27             if let Some(ref mut depth) = data.current_dom_depth {
28             // here if we need it for the post-order one :)                                                                                               28                 *depth -= 1;
                                                                                                                                                             29             }

