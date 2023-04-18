File_Code/rust/9a4f0abd7a/mod/mod_after.rs --- Rust
614             ty::ReStatic |                                                                                                                               614             ty::ReStatic => return r,
615                                                                                                                                                              
616             // ignore `ReScope`, which may appear in impl Trait in bindings.                                                                                 
617             ty::ReScope(..) => return r,                                                                                                                     

