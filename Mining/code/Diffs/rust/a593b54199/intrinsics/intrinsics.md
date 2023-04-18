File_Code/rust/a593b54199/intrinsics/intrinsics_after.rs --- Text (30 errors, exceeded DFT_PARSE_ERROR_LIMIT)
259                 // and not NULL, their offset is 0.                                                                                                      259                 // and not NULL, we pretend there is an allocation of size 0 right there,
...                                                                                                                                                          260                 // and their offset is 0. (There's never a valid object at NULL, making it an
...                                                                                                                                                          261                 // exception from the exception.)
260                 // This is the dual to the special exception for offset-by-0                                                                             262                 // This is the dual to the special exception for offset-by-0
261                 // in the inbounds pointer offset operation.                                                                                             263                 // in the inbounds pointer offset operation (see the Miri code, `src/operator.rs`).

