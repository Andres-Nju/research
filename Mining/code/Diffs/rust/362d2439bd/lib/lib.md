File_Code/rust/362d2439bd/lib/lib_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1700         let segments = iter::once(keywords::PathRoot.ident())                                                                                           1700         let root = if crate_root.is_some() {
                                                                                                                                                             1701             keywords::PathRoot
                                                                                                                                                             1702         } else {
                                                                                                                                                             1703             keywords::Crate
                                                                                                                                                             1704         };
                                                                                                                                                             1705         let segments = iter::once(root.ident())

