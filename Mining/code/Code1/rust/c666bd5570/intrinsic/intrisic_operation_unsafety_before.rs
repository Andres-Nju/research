pub fn intrisic_operation_unsafety(intrinsic: &str) -> hir::Unsafety {
    match intrinsic {
        "size_of" | "min_align_of" | "needs_drop" |
        "add_with_overflow" | "sub_with_overflow" | "mul_with_overflow" |
        "wrapping_add" | "wrapping_sub" | "wrapping_mul" |
        "saturating_add" | "saturating_sub" |
        "rotate_left" | "rotate_right" |
        "ctpop" | "ctlz" | "cttz" | "bswap" | "bitreverse" |
        "minnumf32" | "minnumf64" | "maxnumf32" | "maxnumf64" | "type_name"
        => hir::Unsafety::Normal,
        _ => hir::Unsafety::Unsafe,
    }
}
