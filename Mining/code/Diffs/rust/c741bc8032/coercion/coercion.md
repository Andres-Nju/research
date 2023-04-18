File_Code/rust/c741bc8032/coercion/coercion_after.rs --- 1/2 --- Rust
1107             fcx.eq_types(true, cause, expression_ty, self.merged_ty())                                                                                  1107             fcx.eq_types(label_expression_as_expected, cause, expression_ty, self.merged_ty())

File_Code/rust/c741bc8032/coercion/coercion_after.rs --- 2/2 --- Rust
                                                                                                                                                             1131                     // In the case where this is a "forced unit", like
                                                                                                                                                             1132                     // `break`, we want to call the `()` "expected"
                                                                                                                                                             1133                     // since it is implied by the syntax.
                                                                                                                                                             1134                     // (Note: not all force-units work this way.)"

