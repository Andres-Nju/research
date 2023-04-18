File_Code/rust/098c9b69e1/lib/lib_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                          2946         let mut seen_modules = FxHashSet();

File_Code/rust/098c9b69e1/lib/lib_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2992                         if !worklist.iter().any(|&(m, ..)| m.def() == module.def()) {                                                                   2993                         if seen_modules.insert(module.def_id().unwrap()) {

