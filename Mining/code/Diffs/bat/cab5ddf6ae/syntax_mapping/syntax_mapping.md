File_Code/bat/cab5ddf6ae/syntax_mapping/syntax_mapping_after.rs --- Rust
 97         let canddidate_filename = path.as_ref().file_name().map(Candidate::new);                                                                          97         let candidate_filename = path.as_ref().file_name().map(Candidate::new);
 98         for (ref glob, ref syntax) in self.mappings.iter().rev() {                                                                                        98         for (ref glob, ref syntax) in self.mappings.iter().rev() {
 99             if glob.is_match_candidate(&candidate)                                                                                                        99             if glob.is_match_candidate(&candidate)
100                 || canddidate_filename                                                                                                                   100                 || candidate_filename

