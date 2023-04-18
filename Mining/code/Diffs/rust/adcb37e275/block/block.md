File_Code/rust/adcb37e275/block/block_after.rs --- Rust
                                                                                                                                                           146             // If a block has no trailing expression, then it is given an implicit return type.
                                                                                                                                                           147             // This return type is usually `()`, unless the block is diverging, in which case the
                                                                                                                                                           148             // return type is `!`. For the unit type, we need to actually return the unit, but in
                                                                                                                                                           149             // the case of `!`, no return value is required, as the block will never return.

