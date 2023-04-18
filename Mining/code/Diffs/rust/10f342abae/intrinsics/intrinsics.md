File_Code/rust/10f342abae/intrinsics/intrinsics_after.rs --- 1/2 --- Rust
413         // First, check x % y != 0.                                                                                                                      413         // First, check x % y != 0 (or if that computation overflows).
...                                                                                                                                                          414         let (res, overflow, _ty) = self.overflowing_binary_op(BinOp::Rem, a, b)?;
414         if self.binary_op(BinOp::Rem, a, b)?.to_bits()? != 0 {                                                                                           415         if overflow || res.to_bits(a.layout.size)? != 0 {

File_Code/rust/10f342abae/intrinsics/intrinsics_after.rs --- 2/2 --- Rust
                                                                                                                                                             425         // `Rem` says this is all right, so we can let `Div` do its job.

