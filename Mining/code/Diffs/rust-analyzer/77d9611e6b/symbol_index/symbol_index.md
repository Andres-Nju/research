File_Code/rust-analyzer/77d9611e6b/symbol_index/symbol_index_after.rs --- 1/2 --- Rust
 9 //! finite state machine describing this set of strtings. The strings which                                                                                9 //! finite state machine describing this set of strings. The strings which
10 //! could fuzzy-match a pattern can also be described by a finite state machine.                                                                          10 //! could fuzzy-match a pattern can also be described by a finite state machine.
11 //! What is freakingly cool is that you can now traverse both state machines in                                                                           11 //! What is freakingly cool is that you can now traverse both state machines in
12 //! lock-step to enumerate the strings which are both in the input set and                                                                                12 //! lock-step to enumerate the strings which are both in the input set and
13 //! fuzz-match the query. Or, more formally, given two langauges described by                                                                             13 //! fuzz-match the query. Or, more formally, given two languages described by

File_Code/rust-analyzer/77d9611e6b/symbol_index/symbol_index_after.rs --- 2/2 --- Rust
20 //! file in the current workspace, and run a query aginst the union of all                                                                                20 //! file in the current workspace, and run a query against the union of all
21 //! thouse fsts.                                                                                                                                          21 //! those fsts.

