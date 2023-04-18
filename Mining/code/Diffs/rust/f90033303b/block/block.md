File_Code/rust/f90033303b/block/block_after.rs --- 1/2 --- Rust
433                         self.trans_transmute(&bx, &args[0], dest);                                                                                       433                         self.codegen_transmute(&bx, &args[0], dest);

File_Code/rust/f90033303b/block/block_after.rs --- 2/2 --- Rust
                                                                                                                                                             442                         assert_eq!(bx.cx.layout_of(sig.output()).abi, layout::Abi::Uninhabited);

