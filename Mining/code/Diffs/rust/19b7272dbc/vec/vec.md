File_Code/rust/19b7272dbc/vec/vec_after.rs --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
278 /// this a side-effect that must be preserved.                                                                                                           278 /// this a side-effect that must be preserved. There is one case which we will
                                                                                                                                                             279 /// not break, however: using `unsafe` code to write to the excess capacity,
                                                                                                                                                             280 /// and then increasing the length to match, is always valid.

