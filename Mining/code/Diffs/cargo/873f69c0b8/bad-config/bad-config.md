File_Code/cargo/873f69c0b8/bad-config/bad-config_after.rs --- Rust
381                 execs().with_status(101).with_stderr("\                                                                                                  381                 execs().with_status(101).with_stderr("\
382 [UPDATING] git repository `file:///`                                                                                                                     382 [UPDATING] git repository `file:///`
383 [ERROR] failed to load source for a dependency on `foo`                                                                                                  383 [ERROR] failed to load source for a dependency on `foo`
384                                                                                                                                                          384 
385 Caused by:                                                                                                                                               385 Caused by:
386   Unable to update file:///                                                                                                                              386   Unable to update file:///
387                                                                                                                                                          387 
388 Caused by:                                                                                                                                               388 Caused by:
389   failed to clone into: [..]                                                                                                                             389   failed to clone into: [..]
390                                                                                                                                                          390 
391 Caused by:                                                                                                                                               391 Caused by:
392   [[..]] 'file:///' is not a valid local file URI                                                                                                        392   [..]'file:///' is not a valid local file URI[..]
393 "));                                                                                                                                                     393 "));

