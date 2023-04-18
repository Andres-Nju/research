File_Code/rust/336b902f66/traits/traits_after.rs --- Rust
143         let fn_sig = drop_instance.monomorphic_ty(*self.tcx).fn_sig(*self.tcx);                                                                          143         let fn_sig = drop_instance.ty_env(*self.tcx, self.param_env).fn_sig(*self.tcx);

