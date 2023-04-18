File_Code/rust-analyzer/72792f6bc5/extensions/extensions_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
239         self.syntax()                                                                                                                                    239         self.syntax().children_with_tokens().find(|t| t.kind() == EQ).and_then(|it| it.into_token())
240             .descendants_with_tokens()                                                                                                                       
241             .find(|t| t.kind() == EQ)                                                                                                                        
242             .and_then(|it| it.into_token())                                                                                                                  

