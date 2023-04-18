    fn validate_scalar_layout(
        &self,
        value: ScalarMaybeUndef<M::PointerTag>,
        size: Size,
        path: &Vec<PathElem>,
        layout: &layout::Scalar,
    ) -> EvalResult<'tcx> {
        let (lo, hi) = layout.valid_range.clone().into_inner();
        let max_hi = u128::max_value() >> (128 - size.bits()); // as big as the size fits
        assert!(hi <= max_hi);
        // We could also write `(hi + 1) % (max_hi + 1) == lo` but `max_hi + 1` overflows for `u128`
        if (lo == 0 && hi == max_hi) || (hi + 1 == lo) {
            // Nothing to check
            return Ok(());
        }
        // At least one value is excluded. Get the bits.
        let value = try_validation!(value.not_undef(),
            scalar_format(value), path, format!("something in the range {:?}", layout.valid_range));
        let bits = match value {
            Scalar::Ptr(ptr) => {
                if lo == 1 && hi == max_hi {
                    // only NULL is not allowed.
                    // We can call `check_align` to check non-NULL-ness, but have to also look
                    // for function pointers.
                    let non_null =
                        self.memory.check_align(
                            Scalar::Ptr(ptr), Align::from_bytes(1, 1).unwrap()
                        ).is_ok() ||
                        self.memory.get_fn(ptr).is_ok();
                    if !non_null {
                        // could be NULL
                        return validation_failure!("a potentially NULL pointer", path);
                    }
                    return Ok(());
                } else {
                    // Conservatively, we reject, because the pointer *could* have this
                    // value.
                    return validation_failure!(
                        "a pointer",
                        path,
                        format!(
                            "something that cannot possibly be outside the (wrapping) range {:?}",
                            layout.valid_range
                        )
                    );
                }
            }
            Scalar::Bits { bits, size: value_size } => {
                assert_eq!(value_size as u64, size.bytes());
                bits
            }
        };
        // Now compare. This is slightly subtle because this is a special "wrap-around" range.
        use std::ops::RangeInclusive;
        let in_range = |bound: RangeInclusive<u128>| bound.contains(&bits);
        if lo > hi {
            // wrapping around
            if in_range(0..=hi) || in_range(lo..=max_hi) {
                Ok(())
            } else {
                validation_failure!(
                    bits,
                    path,
                    format!("something in the range {:?} or {:?}", 0..=hi, lo..=max_hi)
                )
            }
        } else {
            if in_range(layout.valid_range.clone()) {
                Ok(())
            } else {
                validation_failure!(
                    bits,
                    path,
                    if hi == max_hi {
                        format!("something greater or equal to {}", lo)
                    } else {
                        format!("something in the range {:?}", layout.valid_range)
                    }
                )
            }
        }
    }

    /// This function checks the data at `op`.  `op` is assumed to cover valid memory if it
    /// is an indirect operand.
    /// It will error if the bits at the destination do not match the ones described by the layout.
    /// The `path` may be pushed to, but the part that is present when the function
    /// starts must not be changed!
    ///
    /// `ref_tracking` can be None to avoid recursive checking below references.
    /// This also toggles between "run-time" (no recursion) and "compile-time" (with recursion)
    /// validation (e.g., pointer values are fine in integers at runtime).
    pub fn validate_operand(
        &self,
        dest: OpTy<'tcx, M::PointerTag>,
        path: &mut Vec<PathElem>,
        mut ref_tracking: Option<&mut RefTracking<'tcx, M::PointerTag>>,
        const_mode: bool,
    ) -> EvalResult<'tcx> {
        trace!("validate_operand: {:?}, {:?}", *dest, dest.layout.ty);

        // If this is a multi-variant layout, we have find the right one and proceed with that.
        // (No good reasoning to make this recursion, but it is equivalent to that.)
        let dest = match dest.layout.variants {
            layout::Variants::NicheFilling { .. } |
            layout::Variants::Tagged { .. } => {
                let variant = match self.read_discriminant(dest) {
                    Ok(res) => res.1,
                    Err(err) => match err.kind {
                        EvalErrorKind::InvalidDiscriminant(val) =>
                            return validation_failure!(
                                format!("invalid enum discriminant {}", val), path
                            ),
                        _ =>
                            return validation_failure!(
                                format!("non-integer enum discriminant"), path
                            ),
                    }
                };
                // Put the variant projection onto the path, as a field
                path.push(PathElem::Field(dest.layout.ty
                                          .ty_adt_def()
                                          .unwrap()
                                          .variants[variant].name));
                // Proceed with this variant
                let dest = self.operand_downcast(dest, variant)?;
                trace!("variant layout: {:#?}", dest.layout);
                dest
            },
            layout::Variants::Single { .. } => dest,
        };

        // First thing, find the real type:
        // If it is a trait object, switch to the actual type that was used to create it.
        let dest = match dest.layout.ty.sty {
            ty::Dynamic(..) => {
                let dest = dest.to_mem_place(); // immediate trait objects are not a thing
                self.unpack_dyn_trait(dest)?.1.into()
            },
            _ => dest
        };

        // If this is a scalar, validate the scalar layout.
        // Things can be aggregates and have scalar layout at the same time, and that
        // is very relevant for `NonNull` and similar structs: We need to validate them
        // at their scalar layout *before* descending into their fields.
        // FIXME: We could avoid some redundant checks here. For newtypes wrapping
        // scalars, we do the same check on every "level" (e.g. first we check
        // MyNewtype and then the scalar in there).
        match dest.layout.abi {
            layout::Abi::Uninhabited =>
                return validation_failure!("a value of an uninhabited type", path),
            layout::Abi::Scalar(ref layout) => {
                let value = try_validation!(self.read_scalar(dest),
                            "uninitialized or unrepresentable data", path);
                self.validate_scalar_layout(value, dest.layout.size, &path, layout)?;
            }
            // FIXME: Should we do something for ScalarPair? Vector?
            _ => {}
        }

        // Check primitive types.  We do this after checking the scalar layout,
        // just to have that done as well.  Primitives can have varying layout,
        // so we check them separately and before aggregate handling.
        // It is CRITICAL that we get this check right, or we might be
        // validating the wrong thing!
        let primitive = match dest.layout.fields {
            // Primitives appear as Union with 0 fields -- except for fat pointers.
            layout::FieldPlacement::Union(0) => true,
            _ => dest.layout.ty.builtin_deref(true).is_some(),
        };
        if primitive {
            let value = try_validation!(self.read_value(dest),
                "uninitialized or unrepresentable data", path);
            return self.validate_primitive_type(
                value,
                &path,
                ref_tracking,
                const_mode,
            );
        }

        // Validate all fields of compound data structures
        let path_len = path.len(); // Remember the length, in case we need to truncate
        match dest.layout.fields {
            layout::FieldPlacement::Union(fields) => {
                // Empty unions are not accepted by rustc. That's great, it means we can
                // use that as an unambiguous signal for detecting primitives.  Make sure
                // we did not miss any primitive.
                debug_assert!(fields > 0);
                // We can't check unions, their bits are allowed to be anything.
                // The fields don't need to correspond to any bit pattern of the union's fields.
                // See https://github.com/rust-lang/rust/issues/32836#issuecomment-406875389
            },
            layout::FieldPlacement::Arbitrary { ref offsets, .. } => {
                // Go look at all the fields
                for i in 0..offsets.len() {
                    let field = self.operand_field(dest, i as u64)?;
                    path.push(self.aggregate_field_path_elem(dest.layout, i));
                    self.validate_operand(
                        field,
                        path,
                        ref_tracking.as_mut().map(|r| &mut **r),
                        const_mode,
                    )?;
                    path.truncate(path_len);
                }
            }
            layout::FieldPlacement::Array { stride, .. } => {
                let dest = if dest.layout.is_zst() {
                    // it's a ZST, the memory content cannot matter
                    MPlaceTy::dangling(dest.layout, self)
                } else {
                    // non-ZST array/slice/str cannot be immediate
                    dest.to_mem_place()
                };
                match dest.layout.ty.sty {
                    // Special handling for strings to verify UTF-8
                    ty::Str => {
                        try_validation!(self.read_str(dest),
                            "uninitialized or non-UTF-8 data in str", path);
                    }
                    // Special handling for arrays/slices of builtin integer types
                    ty::Array(tys, ..) | ty::Slice(tys) if {
                        // This optimization applies only for integer and floating point types
                        // (i.e., types that can hold arbitrary bytes).
                        match tys.sty {
                            ty::Int(..) | ty::Uint(..) | ty::Float(..) => true,
                            _ => false,
                        }
                    } => {
                        // This is the length of the array/slice.
                        let len = dest.len(self)?;
                        // Since primitive types are naturally aligned and tightly packed in arrays,
                        // we can use the stride to get the size of the integral type.
                        let ty_size = stride.bytes();
                        // This is the size in bytes of the whole array.
                        let size = Size::from_bytes(ty_size * len);

                        // NOTE: Keep this in sync with the handling of integer and float
                        // types above, in `validate_primitive_type`.
                        // In run-time mode, we accept pointers in here.  This is actually more
                        // permissive than a per-element check would be, e.g. we accept
                        // an &[u8] that contains a pointer even though bytewise checking would
                        // reject it.  However, that's good: We don't inherently want
                        // to reject those pointers, we just do not have the machinery to
                        // talk about parts of a pointer.
                        // We also accept undef, for consistency with the type-based checks.
                        match self.memory.check_bytes(
                            dest.ptr,
                            size,
                            /*allow_ptr_and_undef*/!const_mode,
                        ) {
                            // In the happy case, we needn't check anything else.
                            Ok(()) => {},
                            // Some error happened, try to provide a more detailed description.
                            Err(err) => {
                                // For some errors we might be able to provide extra information
                                match err.kind {
                                    EvalErrorKind::ReadUndefBytes(offset) => {
                                        // Some byte was undefined, determine which
                                        // element that byte belongs to so we can
                                        // provide an index.
                                        let i = (offset.bytes() / ty_size) as usize;
                                        path.push(PathElem::ArrayElem(i));

                                        return validation_failure!(
                                            "undefined bytes", path
                                        )
                                    },
                                    // Other errors shouldn't be possible
                                    _ => return Err(err),
                                }
                            }
                        }
                    },
                    _ => {
                        // This handles the unsized case correctly as well, as well as
                        // SIMD an all sorts of other array-like types.
                        for (i, field) in self.mplace_array_fields(dest)?.enumerate() {
                            let field = field?;
                            path.push(PathElem::ArrayElem(i));
                            self.validate_operand(
                                field.into(),
                                path,
                                ref_tracking.as_mut().map(|r| &mut **r),
                                const_mode,
                            )?;
                            path.truncate(path_len);
                        }
                    }
                }
            },
        }
        Ok(())
    }
