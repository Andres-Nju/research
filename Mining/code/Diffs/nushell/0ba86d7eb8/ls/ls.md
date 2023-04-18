File_Code/nushell/0ba86d7eb8/ls/ls_after.rs --- Rust
454         let zero_sized =                                                                                                                                 454         let zero_sized = file_type == "pipe"
455             file_type == "socket" || file_type == "block device" || file_type == "char device";                                                          455             || file_type == "socket"
                                                                                                                                                             456             || file_type == "char device"
                                                                                                                                                             457             || file_type == "block device";

