File_Code/nushell/5e177fe8e7/file_completions/file_completions_after.rs --- Rust
                                                                                                                                                           101     // Replace base filter with no filter once all the results are already based in the current path
                                                                                                                                                           102     fn filter(&self, _: Vec<u8>, items: Vec<Suggestion>, _: CompletionOptions) -> Vec<Suggestion> {
                                                                                                                                                           103         items
                                                                                                                                                           104     }

