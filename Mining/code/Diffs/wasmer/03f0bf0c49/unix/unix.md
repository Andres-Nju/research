File_Code/wasmer/03f0bf0c49/unix/unix_after.rs --- Rust
  .                                                                                                                                                          773         #[cfg(target_os = "macos")]
773         let stat_ptr = &mut stat as *mut stat as *mut c_void;                                                                                            774         let stat_ptr = &mut stat as *mut stat as *mut c_void;
                                                                                                                                                             775         #[cfg(not(target_os = "macos"))]
                                                                                                                                                             776         let stat_ptr = &mut stat as *mut stat;

