File_Code/deno/944d708e3c/repl_tests/repl_tests_after.rs --- 1/2 --- Rust
189   // does not panic when tabbing when empty                                                                                                                
190   util::with_pty(&["repl"], |mut console| {                                                                                                                
191     console.write_line("import '\t");                                                                                                                      
192   });                                                                                                                                                      

File_Code/deno/944d708e3c/repl_tests/repl_tests_after.rs --- 2/2 --- Rust
                                                                                                                                                             190 #[test]
                                                                                                                                                             191 fn pty_complete_imports_no_panic_empty_specifier() {
                                                                                                                                                             192   // does not panic when tabbing when empty
                                                                                                                                                             193   util::with_pty(&["repl"], |mut console| {
                                                                                                                                                             194     console.write_line("import '\t';");
                                                                                                                                                             195     console.write_line("close();");
                                                                                                                                                             196   });
                                                                                                                                                             197 }

