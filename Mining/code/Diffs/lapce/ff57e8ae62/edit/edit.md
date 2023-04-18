File_Code/lapce/ff57e8ae62/edit/edit_after.rs --- 1/2 --- Rust
107         // We have to keep track of a shift because the deletions aren't properly moved forward                                                            . 
108         let mut shift = insertions.inserts_len();                                                                                                          . 
109         for (start, end) in deletions.range_iter(CountMatcher::NonZero) {                                                                                107         for (start, end) in deletions.range_iter(CountMatcher::NonZero) {
110             edits.push(create_delete_edit(&text, start + shift, end + shift));                                                                           108             edits.push(create_delete_edit(&text, start, end));
111                                                                                                                                                          109 
112             let delete_delta = RopeDelta::simple_edit(                                                                                                   110             let delete_delta = RopeDelta::simple_edit(
113                 Interval::new(start + shift, end + shift),                                                                                               111                 Interval::new(start, end),

File_Code/lapce/ff57e8ae62/edit/edit_after.rs --- 2/2 --- Rust
118             shift -= end - start;                                                                                                                            

