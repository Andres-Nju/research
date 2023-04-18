    fn visit_variant(
        &mut self,
        old_op: OpTy<'tcx, M::PointerTag>,
        variant_id: VariantIdx,
        new_op: OpTy<'tcx, M::PointerTag>
    ) -> EvalResult<'tcx> {
        let name = match old_op.layout.ty.sty {
            ty::Adt(adt, _) => PathElem::Variant(adt.variants[variant_id].ident.name),
            // Generators also have variants
            ty::Generator(..) => PathElem::GeneratoreState(variant_id),
            _ => bug!("Unexpected type with variant: {:?}", old_op.layout.ty),
        };
        self.visit_elem(new_op, name)
    }

    #[inline]
    fn visit_value(&mut self, op: OpTy<'tcx, M::PointerTag>) -> EvalResult<'tcx>
    {
        trace!("visit_value: {:?}, {:?}", *op, op.layout);
        // Translate some possible errors to something nicer.
        match self.walk_value(op) {
            Ok(()) => Ok(()),
            Err(err) => match err.kind {
                InterpError::InvalidDiscriminant(val) =>
                    validation_failure!(
                        val, self.path, "a valid enum discriminant"
                    ),
                InterpError::ReadPointerAsBytes =>
                    validation_failure!(
                        "a pointer", self.path, "plain (non-pointer) bytes"
                    ),
                _ => Err(err),
            }
        }
    }

    fn visit_primitive(&mut self, value: OpTy<'tcx, M::PointerTag>) -> EvalResult<'tcx>
    {
        let value = self.ecx.read_immediate(value)?;
        // Go over all the primitive types
        let ty = value.layout.ty;
        match ty.sty {
            ty::Bool => {
                let value = value.to_scalar_or_undef();
                try_validation!(value.to_bool(),
                    value, self.path, "a boolean");
            },
            ty::Char => {
                let value = value.to_scalar_or_undef();
                try_validation!(value.to_char(),
                    value, self.path, "a valid unicode codepoint");
            },
            ty::Float(_) | ty::Int(_) | ty::Uint(_) => {
                // NOTE: Keep this in sync with the array optimization for int/float
                // types below!
                let size = value.layout.size;
                let value = value.to_scalar_or_undef();
                if self.const_mode {
                    // Integers/floats in CTFE: Must be scalar bits, pointers are dangerous
                    try_validation!(value.to_bits(size),
                        value, self.path, "initialized plain (non-pointer) bytes");
                } else {
                    // At run-time, for now, we accept *anything* for these types, including
                    // undef. We should fix that, but let's start low.
                }
            }
            ty::RawPtr(..) => {
                if self.const_mode {
                    // Integers/floats in CTFE: For consistency with integers, we do not
                    // accept undef.
                    let _ptr = try_validation!(value.to_scalar_ptr(),
                        "undefined address in raw pointer", self.path);
                    let _meta = try_validation!(value.to_meta(),
                        "uninitialized data in raw fat pointer metadata", self.path);
                } else {
                    // Remain consistent with `usize`: Accept anything.
                }
            }
