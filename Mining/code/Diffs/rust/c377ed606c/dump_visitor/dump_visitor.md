File_Code/rust/c377ed606c/dump_visitor/dump_visitor_after.rs --- Rust
869                     Some(ty) => ty.ty_adt_def().unwrap(),                                                                                                869                     Some(ty) if ty.ty_adt_def().is_some() => ty.ty_adt_def().unwrap(),
870                     None => {                                                                                                                            870                     _ => {

