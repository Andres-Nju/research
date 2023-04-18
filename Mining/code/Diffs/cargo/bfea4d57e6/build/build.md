File_Code/cargo/bfea4d57e6/build/build_after.rs --- 1/2 --- Rust
1059             "\                                                                                                                                          1059             "\
1060 error: no matching package named `not_cached_dep` found                                                                                                 1060 error: no matching package named `not_cached_dep` found
1061 location searched: registry `[..]`                                                                                                                      1061 location searched: registry `[..]`
1062 required by package `bar v0.1.0 ([..])`                                                                                                                 1062 required by package `bar v0.1.0 ([..])`
1063 As a reminder, you're using offline mode (-Z offline) \                                                                                                 1063 As a reminder, you're using offline mode (-Z offline) \
1064 which can sometimes cause surprising resolution failures, \                                                                                             1064 which can sometimes cause surprising resolution failures, \
1065 if this error is too confusing you may with to retry \                                                                                                  1065 if this error is too confusing you may wish to retry \
1066 without the offline flag.",                                                                                                                             1066 without the offline flag.",

File_Code/cargo/bfea4d57e6/build/build_after.rs --- 2/2 --- Rust
1284             "\                                                                                                                                          1284             "\
1285 error: no matching package named `baz` found                                                                                                            1285 error: no matching package named `baz` found
1286 location searched: registry `[..]`                                                                                                                      1286 location searched: registry `[..]`
1287 required by package `bar v0.1.0`                                                                                                                        1287 required by package `bar v0.1.0`
1288     ... which is depended on by `foo v0.0.1 ([CWD])`                                                                                                    1288     ... which is depended on by `foo v0.0.1 ([CWD])`
1289 As a reminder, you're using offline mode (-Z offline) \                                                                                                 1289 As a reminder, you're using offline mode (-Z offline) \
1290 which can sometimes cause surprising resolution failures, \                                                                                             1290 which can sometimes cause surprising resolution failures, \
1291 if this error is too confusing you may with to retry \                                                                                                  1291 if this error is too confusing you may wish to retry \
1292 without the offline flag.",                                                                                                                             1292 without the offline flag.",

