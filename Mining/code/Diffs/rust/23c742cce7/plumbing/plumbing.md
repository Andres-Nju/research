File_Code/rust/23c742cce7/plumbing/plumbing_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
746             pub fn record_query_hits(&self, sess: &Session) {                                                                                            746             pub fn record_computed_queries(&self, sess: &Session) {
747                 sess.profiler(|p| {                                                                                                                      747                 sess.profiler(|p| {
748                     $(                                                                                                                                   748                     $(
749                         p.record_queries(                                                                                                                749                         p.record_computed_queries(

