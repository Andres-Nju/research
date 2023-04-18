File_Code/rust/6a3dfa4dba/lib/lib_after.rs --- Rust
1008             Def::PrimTy(..) | Def::SelfTy(..) => return false,                                                                                          1008             Def::PrimTy(..) | Def::SelfTy(..) | Def::Err => return false,

