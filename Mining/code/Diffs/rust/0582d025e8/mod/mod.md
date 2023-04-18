File_Code/rust/0582d025e8/mod/mod_after.rs --- 1/2 --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1039     let ret_ty = fn_sig.output();                                                                                                                       1039     let declared_ret_ty = fn_sig.output();
1040     fcx.require_type_is_sized(ret_ty, decl.output.span(), traits::SizedReturnType);                                                                     1040     fcx.require_type_is_sized(declared_ret_ty, decl.output.span(), traits::SizedReturnType);
1041     let revealed_ret_ty = fcx.instantiate_anon_types_from_return_value(fn_id, &ret_ty);                                                                 1041     let revealed_ret_ty = fcx.instantiate_anon_types_from_return_value(fn_id, &declared_ret_ty);

File_Code/rust/0582d025e8/mod/mod_after.rs --- 2/2 --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1135                         let substs = fcx.tcx.mk_substs(iter::once(Kind::from(ret_ty)));                                                                 1135                         let substs = fcx.tcx.mk_substs(iter::once(Kind::from(declared_ret_ty)));

