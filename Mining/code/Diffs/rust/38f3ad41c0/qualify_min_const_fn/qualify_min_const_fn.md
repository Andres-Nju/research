File_Code/rust/38f3ad41c0/qualify_min_const_fn/qualify_min_const_fn_after.rs --- 1/2 --- Text (30 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                           167         Rvalue::Cast(CastKind::ClosureFnPointer, _, _) |

File_Code/rust/38f3ad41c0/qualify_min_const_fn/qualify_min_const_fn_after.rs --- 2/2 --- Text (30 errors, exceeded DFT_PARSE_ERROR_LIMIT)
171         Rvalue::Cast(CastKind::ClosureFnPointer, _, _) => Err((                                                                                              
172             span,                                                                                                                                            
173             "closures are not allowed in const fn".into(),                                                                                                   
174         )),                                                                                                                                                  

