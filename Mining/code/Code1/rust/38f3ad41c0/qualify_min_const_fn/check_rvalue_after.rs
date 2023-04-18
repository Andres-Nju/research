fn check_rvalue(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    mir: &'a Mir<'tcx>,
    rvalue: &Rvalue<'tcx>,
    span: Span,
) -> McfResult {
    match rvalue {
        Rvalue::Repeat(operand, _) | Rvalue::Use(operand) => {
            check_operand(tcx, mir, operand, span)
        }
        Rvalue::Len(place) | Rvalue::Discriminant(place) | Rvalue::Ref(_, _, place) => {
            check_place(tcx, mir, place, span, PlaceMode::Read)
        }
        Rvalue::Cast(CastKind::Misc, operand, cast_ty) => {
            use rustc::ty::cast::CastTy;
            let cast_in = CastTy::from_ty(operand.ty(mir, tcx)).expect("bad input type for cast");
            let cast_out = CastTy::from_ty(cast_ty).expect("bad output type for cast");
            match (cast_in, cast_out) {
                (CastTy::Ptr(_), CastTy::Int(_)) | (CastTy::FnPtr, CastTy::Int(_)) => Err((
                    span,
                    "casting pointers to ints is unstable in const fn".into(),
                )),
                (CastTy::RPtr(_), CastTy::Float) => bug!(),
                (CastTy::RPtr(_), CastTy::Int(_)) => bug!(),
                (CastTy::Ptr(_), CastTy::RPtr(_)) => bug!(),
                _ => check_operand(tcx, mir, operand, span),
            }
        }
        Rvalue::Cast(CastKind::UnsafeFnPointer, _, _) |
        Rvalue::Cast(CastKind::ClosureFnPointer, _, _) |
        Rvalue::Cast(CastKind::ReifyFnPointer, _, _) => Err((
            span,
            "function pointer casts are not allowed in const fn".into(),
        )),
        Rvalue::Cast(CastKind::Unsize, _, _) => Err((
            span,
            "unsizing casts are not allowed in const fn".into(),
        )),
        // binops are fine on integers
        Rvalue::BinaryOp(_, lhs, rhs) | Rvalue::CheckedBinaryOp(_, lhs, rhs) => {
            check_operand(tcx, mir, lhs, span)?;
            check_operand(tcx, mir, rhs, span)?;
            let ty = lhs.ty(mir, tcx);
            if ty.is_integral() || ty.is_bool() || ty.is_char() {
                Ok(())
            } else {
                Err((
                    span,
                    "only int, `bool` and `char` operations are stable in const fn".into(),
                ))
            }
        }
        Rvalue::NullaryOp(NullOp::SizeOf, _) => Ok(()),
        Rvalue::NullaryOp(NullOp::Box, _) => Err((
            span,
            "heap allocations are not allowed in const fn".into(),
        )),
        Rvalue::UnaryOp(_, operand) => {
            let ty = operand.ty(mir, tcx);
            if ty.is_integral() || ty.is_bool() {
                check_operand(tcx, mir, operand, span)
            } else {
                Err((
                    span,
                    "only int and `bool` operations are stable in const fn".into(),
                ))
            }
        }
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                check_operand(tcx, mir, operand, span)?;
            }
            Ok(())
        }
    }
}
