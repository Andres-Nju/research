File_Code/rust/403ae37ce8/mod/mod_after.rs --- 1/3 --- Text (12 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1938             let default_map: FxHashMap<_, _> =                                                                                                          1938             let default_map: FxHashMap<Ty<'tcx>, _> =
1939                 unsolved_variables                                                                                                                      1939                 unsolved_variables
1940                     .iter()                                                                                                                             1940                     .iter()
1941                     .filter_map(|t| self.default(t).map(|d| (t, d)))                                                                                    1941                     .filter_map(|t| self.default(t).map(|d| (*t, d)))

File_Code/rust/403ae37ce8/mod/mod_after.rs --- 2/3 --- Text (12 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2071         default_map: &'b FxHashMap<&'b Ty<'tcx>, type_variable::Default<'tcx>>,                                                                         2071         default_map: &'b FxHashMap<Ty<'tcx>, type_variable::Default<'tcx>>,

File_Code/rust/403ae37ce8/mod/mod_after.rs --- 3/3 --- Text (12 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2090                         if let Some(default) = default_map.get(&ty) {                                                                                   2090                         if let Some(default) = default_map.get(ty) {

