File_Code/rust/2da89dea4b/test/test_after.rs --- Rust
1530         try_run(builder, rustbook_cmd.arg("linkcheck").arg(&src));                                                                                      1530         let toolstate = if try_run(builder, rustbook_cmd.arg("linkcheck").arg(&src)) {
                                                                                                                                                             1531             ToolState::TestPass
                                                                                                                                                             1532         } else {
                                                                                                                                                             1533             ToolState::TestFail
                                                                                                                                                             1534         };
                                                                                                                                                             1535         builder.save_toolstate("rustc-guide", toolstate);

