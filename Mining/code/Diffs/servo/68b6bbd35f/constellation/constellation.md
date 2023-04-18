File_Code/servo/68b6bbd35f/constellation/constellation_after.rs --- 1/3 --- Rust
2067         let (window_size, pipeline_id, parent_pipeline_id, is_visible) =                                                                                2067         let (window_size, pipeline_id, parent_pipeline_id, is_private, is_visible) =

File_Code/servo/68b6bbd35f/constellation/constellation_after.rs --- 2/3 --- Rust
                                                                                                                                                             2073                     ctx.is_private,

File_Code/servo/68b6bbd35f/constellation/constellation_after.rs --- 3/3 --- Rust
2144                 // TODO(mandreyel): why is this false? Should we not inherit the                                                                             
2145                 // privacy of the (existing) browsing context?                                                                                               
2146                 let is_private = false;                                                                                                                      

