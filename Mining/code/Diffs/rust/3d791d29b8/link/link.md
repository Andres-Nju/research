File_Code/rust/3d791d29b8/link/link_after.rs --- Rust
647         let msg = "clang: error: unable to execute command: \                                                                                            647         let msg_segv = "clang: error: unable to execute command: Segmentation fault: 11";
648                    Segmentation fault: 11";                                                                                                              648         let msg_bus  = "clang: error: unable to execute command: Bus error: 10";
649         if !out.contains(msg) {                                                                                                                          649         if !(out.contains(msg_segv) || out.contains(msg_bus)) {

