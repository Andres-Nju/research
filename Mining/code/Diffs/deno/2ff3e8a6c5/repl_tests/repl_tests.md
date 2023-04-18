File_Code/deno/2ff3e8a6c5/repl_tests/repl_tests_after.rs --- Rust
220     console.write_line("console.log('🦕');");                                                                                                            220     console.write_line(r#"console.log('\u{1F995}');"#);
221     console.write_line("close();");                                                                                                                      221     console.write_line("close();");
222                                                                                                                                                          222 
223     let output = console.read_all_output();                                                                                                              223     let output = console.read_all_output();
224     // one for input, one for output                                                                                                                     224     // only one for the output (since input is escaped)
225     let emoji_count = output.chars().filter(|c| *c == '🦕').count();                                                                                     225     let emoji_count = output.chars().filter(|c| *c == '🦕').count();
226     assert_eq!(emoji_count, 2);                                                                                                                          226     assert_eq!(emoji_count, 1);

