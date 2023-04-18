File_Code/servo/fc0854c4e2/constellation/constellation_after.rs --- 1/2 --- Rust
                                                                                                                                                           789         debug!("Creating new browsing context {}", browsing_context_id);

File_Code/servo/fc0854c4e2/constellation/constellation_after.rs --- 2/2 --- Rust
2860                 Some((parent_id, _)) => {                                                                                                               2861                 Some((parent_id, _)) => pipeline_id = parent_id,
....                                                                                                                                                         2862                 None => {
2861                     browsing_context_id = pipeline.browsing_context_id;                                                                                 2863                     browsing_context_id = pipeline.browsing_context_id;
2862                     pipeline_id = parent_id;                                                                                                            2864                     break;
2863                 },                                                                                                                                      2865                 },
2864                 None => break,                                                                                                                               

