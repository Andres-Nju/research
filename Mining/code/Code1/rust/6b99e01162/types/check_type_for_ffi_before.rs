    fn check_type_for_ffi(&self,
                          cache: &mut FnvHashSet<Ty<'tcx>>,
                          ty: Ty<'tcx>)
                          -> FfiResult {
        use self::FfiResult::*;
        let cx = self.cx.tcx;

        // Protect against infinite recursion, for example
        // `struct S(*mut S);`.
        // FIXME: A recursion limit is necessary as well, for irregular
        // recusive types.
        if !cache.insert(ty) {
            return FfiSafe;
        }

        match ty.sty {
            ty::TyAdt(def, substs) => match def.adt_kind() {
                AdtKind::Struct => {
                    if !cx.lookup_repr_hints(def.did).contains(&attr::ReprExtern) {
                        return FfiUnsafe(
                            "found struct without foreign-function-safe \
                            representation annotation in foreign module, \
                            consider adding a #[repr(C)] attribute to \
                            the type");
                    }

                    // We can't completely trust repr(C) markings; make sure the
                    // fields are actually safe.
                    if def.struct_variant().fields.is_empty() {
                        return FfiUnsafe(
                            "found zero-size struct in foreign module, consider \
                            adding a member to this struct");
                    }

                    for field in &def.struct_variant().fields {
                        let field_ty = cx.normalize_associated_type(&field.ty(cx, substs));
                        let r = self.check_type_for_ffi(cache, field_ty);
                        match r {
                            FfiSafe => {}
                            FfiBadStruct(..) | FfiBadUnion(..) | FfiBadEnum(..) => { return r; }
                            FfiUnsafe(s) => { return FfiBadStruct(def.did, s); }
                        }
                    }
                    FfiSafe
                }
                AdtKind::Union => {
                    if !cx.lookup_repr_hints(def.did).contains(&attr::ReprExtern) {
                        return FfiUnsafe(
                            "found union without foreign-function-safe \
                            representation annotation in foreign module, \
                            consider adding a #[repr(C)] attribute to \
                            the type");
                    }

                    for field in &def.struct_variant().fields {
                        let field_ty = cx.normalize_associated_type(&field.ty(cx, substs));
                        let r = self.check_type_for_ffi(cache, field_ty);
                        match r {
                            FfiSafe => {}
                            FfiBadStruct(..) | FfiBadUnion(..) | FfiBadEnum(..) => { return r; }
                            FfiUnsafe(s) => { return FfiBadUnion(def.did, s); }
                        }
                    }
                    FfiSafe
                }
                AdtKind::Enum => {
                    if def.variants.is_empty() {
                        // Empty enums are okay... although sort of useless.
                        return FfiSafe
                    }

                    // Check for a repr() attribute to specify the size of the
                    // discriminant.
                    let repr_hints = cx.lookup_repr_hints(def.did);
                    match &repr_hints[..] {
                        &[] => {
                            // Special-case types like `Option<extern fn()>`.
                            if !is_repr_nullable_ptr(cx, def, substs) {
                                return FfiUnsafe(
                                    "found enum without foreign-function-safe \
                                    representation annotation in foreign module, \
                                    consider adding a #[repr(...)] attribute to \
                                    the type")
                            }
                        }
                        &[ref hint] => {
                            if !hint.is_ffi_safe() {
                                // FIXME: This shouldn't be reachable: we should check
                                // this earlier.
                                return FfiUnsafe(
                                    "enum has unexpected #[repr(...)] attribute")
                            }

                            // Enum with an explicitly sized discriminant; either
                            // a C-style enum or a discriminated union.

                            // The layout of enum variants is implicitly repr(C).
                            // FIXME: Is that correct?
                        }
                        _ => {
                            // FIXME: This shouldn't be reachable: we should check
                            // this earlier.
                            return FfiUnsafe(
                                "enum has too many #[repr(...)] attributes");
                        }
                    }

                    // Check the contained variants.
                    for variant in &def.variants {
                        for field in &variant.fields {
                            let arg = cx.normalize_associated_type(&field.ty(cx, substs));
                            let r = self.check_type_for_ffi(cache, arg);
                            match r {
                                FfiSafe => {}
                                FfiBadStruct(..) | FfiBadUnion(..) | FfiBadEnum(..) => { return r; }
                                FfiUnsafe(s) => { return FfiBadEnum(def.did, s); }
                            }
                        }
                    }
                    FfiSafe
                }
            },

            ty::TyChar => {
                FfiUnsafe("found Rust type `char` in foreign module, while \
                           `u32` or `libc::wchar_t` should be used")
            }

            // Primitive types with a stable representation.
            ty::TyBool | ty::TyInt(..) | ty::TyUint(..) |
            ty::TyFloat(..) | ty::TyNever => FfiSafe,

            ty::TyBox(..) => {
                FfiUnsafe("found Rust type Box<_> in foreign module, \
                           consider using a raw pointer instead")
            }

            ty::TySlice(_) => {
                FfiUnsafe("found Rust slice type in foreign module, \
                           consider using a raw pointer instead")
            }

            ty::TyTrait(..) => {
                FfiUnsafe("found Rust trait type in foreign module, \
                           consider using a raw pointer instead")
            }

            ty::TyStr => {
                FfiUnsafe("found Rust type `str` in foreign module; \
                           consider using a `*const libc::c_char`")
            }

            ty::TyTuple(_) => {
                FfiUnsafe("found Rust tuple type in foreign module; \
                           consider using a struct instead`")
            }

            ty::TyRawPtr(ref m) | ty::TyRef(_, ref m) => {
                self.check_type_for_ffi(cache, m.ty)
            }

            ty::TyArray(ty, _) => {
                self.check_type_for_ffi(cache, ty)
            }

            ty::TyFnPtr(bare_fn) => {
                match bare_fn.abi {
                    Abi::Rust |
                    Abi::RustIntrinsic |
                    Abi::PlatformIntrinsic |
                    Abi::RustCall => {
                        return FfiUnsafe(
                            "found function pointer with Rust calling \
                             convention in foreign module; consider using an \
                             `extern` function pointer")
                    }
                    _ => {}
                }

                let sig = cx.erase_late_bound_regions(&bare_fn.sig);
                if !sig.output.is_nil() {
                    let r = self.check_type_for_ffi(cache, sig.output);
                    match r {
                        FfiSafe => {}
                        _ => { return r; }
                    }
                }
                for arg in sig.inputs {
                    let r = self.check_type_for_ffi(cache, arg);
                    match r {
                        FfiSafe => {}
                        _ => { return r; }
                    }
                }
                FfiSafe
            }

            ty::TyParam(..) | ty::TyInfer(..) | ty::TyError |
            ty::TyClosure(..) | ty::TyProjection(..) | ty::TyAnon(..) |
            ty::TyFnDef(..) => {
                bug!("Unexpected type in foreign function")
            }
        }
    }
