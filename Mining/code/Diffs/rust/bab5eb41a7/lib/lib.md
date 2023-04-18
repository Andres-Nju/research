File_Code/rust/bab5eb41a7/lib/lib_after.rs --- 1/4 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
277     delayed_span_bug: Lock<Vec<Diagnostic>>,                                                                                                             277     delayed_span_bugs: Lock<Vec<Diagnostic>>,

File_Code/rust/bab5eb41a7/lib/lib_after.rs --- 2/4 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
309             let mut bugs = self.delayed_span_bug.borrow_mut();                                                                                           309             let mut bugs = self.delayed_span_bugs.borrow_mut();

File_Code/rust/bab5eb41a7/lib/lib_after.rs --- 3/4 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
365             delayed_span_bug: Lock::new(Vec::new()),                                                                                                     365             delayed_span_bugs: Lock::new(Vec::new()),

File_Code/rust/bab5eb41a7/lib/lib_after.rs --- 4/4 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
533         self.delayed_span_bug.borrow_mut().push(diagnostic);                                                                                             533         self.delayed_span_bugs.borrow_mut().push(diagnostic);

