File_Code/deno/b5d3eb5c23/repl_tests/repl_tests_after.rs --- Rust
201     console.write_line("globalThis");                                                                                                                    201     console.write_line("'Length: ' + Object.keys(globalThis).filter(k => k.startsWith('__DENO_')).length;");
202     console.write_line_raw("1 + 256");                                                                                                                   202     console.expect("Length: 0");
203     let output = console.read_until("257");                                                                                                                  
204     assert_contains!(output, "clear:");                                                                                                                      
205     assert_not_contains!(output, "__DENO_");                                                                                                                 

