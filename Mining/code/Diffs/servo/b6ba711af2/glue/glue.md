File_Code/servo/b6ba711af2/glue/glue_after.rs --- Rust
                                                                                                                                                          1125 #[no_mangle]
                                                                                                                                                          1126 pub extern "C" fn Servo_StyleSet_Clear(raw_data: RawServoStyleSetBorrowed) {
                                                                                                                                                          1127     let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
                                                                                                                                                          1128     data.clear_stylist();
                                                                                                                                                          1129 }

