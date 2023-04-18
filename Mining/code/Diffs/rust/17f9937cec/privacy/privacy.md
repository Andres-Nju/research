File_Code/rust/17f9937cec/privacy/privacy_after.rs --- Rust
26     // public, then type `T` is exported. Its values can be obtained by other crates                                                                      26     // public, then type `T` is reachable. Its values can be obtained by other crates
27     // even if the type itseld is not nameable.                                                                                                           27     // even if the type itself is not nameable.
28     // FIXME: Mostly unimplemented. Only `type` aliases export items currently.                                                                              

