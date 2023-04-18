File_Code/servo/26ad2a1331/image/image_after.rs --- Rust
436                                 let ordering = a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal);                                                         436                                 return a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal);
437                                 if ordering != Ordering::Equal {                                                                                             
438                                     return ordering;                                                                                                         
439                                 }                                                                                                                            

