File_Code/gfx/a6cfbdf14c/main/main_after.rs --- Rust
180         println!("total time:\t\t{0:4.2}ms", duration_to_f64(swap));                                                                                     180         println!("total time:\t\t{0:4.2}ms", duration_to_ms(swap));
181         println!("\tcreate list:\t{0:4.2}ms", duration_to_f64(pre_submit));                                                                              181         println!("\tcreate list:\t{0:4.2}ms", duration_to_ms(pre_submit));
182         println!("\tsubmit:\t\t{0:4.2}ms", duration_to_f64(post_submit - pre_submit));                                                                   182         println!("\tsubmit:\t\t{0:4.2}ms", duration_to_ms(post_submit - pre_submit));
183         println!("\tgpu wait:\t{0:4.2}ms", duration_to_f64(swap - post_submit));                                                                         183         println!("\tgpu wait:\t{0:4.2}ms", duration_to_ms(swap - post_submit));

