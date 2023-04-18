    pub fn read_drop_type_from_vtable(
        &self,
        vtable: Scalar<M::PointerTag>,
    ) -> InterpResult<'tcx, (ty::Instance<'tcx>, Ty<'tcx>)> {
        // We don't care about the pointee type; we just want a pointer.
        let vtable = self
            .memory
            .check_ptr_access(
                vtable,
                self.tcx.data_layout.pointer_size,
                self.tcx.data_layout.pointer_align.abi,
            )?
            .expect("cannot be a ZST");
        let drop_fn =
            self.memory.get_raw(vtable.alloc_id)?.read_ptr_sized(self, vtable)?.not_undef()?;
        // We *need* an instance here, no other kind of function value, to be able
        // to determine the type.
        let drop_instance = self.memory.get_fn(drop_fn)?.as_instance()?;
        trace!("Found drop fn: {:?}", drop_instance);
        let fn_sig = drop_instance.monomorphic_ty(*self.tcx).fn_sig(*self.tcx);
        let fn_sig = self.tcx.normalize_erasing_late_bound_regions(self.param_env, &fn_sig);
        // The drop function takes `*mut T` where `T` is the type being dropped, so get that.
        let args = fn_sig.inputs();
        if args.len() != 1 {
            throw_ub_format!("drop fn should have 1 argument, but signature is {:?}", fn_sig);
        }
