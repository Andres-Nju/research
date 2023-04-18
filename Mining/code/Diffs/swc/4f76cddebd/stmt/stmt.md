File_Code/swc/4f76cddebd/stmt/stmt_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
  .                                                                                                                                                          878         let await_start = cur_pos!();
878         let await_token = if eat!("await") {                                                                                                             879         let await_token = if eat!("await") {
879             Some(span!(start))                                                                                                                           880             Some(span!(await_start))

