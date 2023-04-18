File_Code/rust/077ba8a799/lowering/lowering_after.rs --- 1/2 --- Text (10 errors, exceeded DFT_PARSE_ERROR_LIMIT)
   .                                                                                                                                                          998                         // Set the name to `impl Bound1 + Bound2`
   .                                                                                                                                                          999                         let name = Symbol::intern(&pprust::ty_to_string(t));
 998                         self.in_band_ty_params.push(hir::TyParam {                                                                                      1000                         self.in_band_ty_params.push(hir::TyParam {
 999                             // Set the name to `impl Bound1 + Bound2`                                                                                   1001                             name,
1000                             name: Symbol::intern(&pprust::ty_to_string(t)),                                                                                  

File_Code/rust/077ba8a799/lowering/lowering_after.rs --- 2/2 --- Text (10 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1012                             segments: vec![].into(),                                                                                                    1013                             segments: hir_vec![hir::PathSegment::from_name(name)],

