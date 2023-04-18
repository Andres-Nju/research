File_Code/rust/ab0092e0d4/tool/tool_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
278         #[derive(Copy, Clone)]                                                                                                                           278         #[derive(Copy, PartialEq, Eq, Clone)]

File_Code/rust/ab0092e0d4/tool/tool_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
643             if compiler.stage == 0 {                                                                                                                     643             if compiler.stage == 0 && tool != Tool::ErrorIndex {

