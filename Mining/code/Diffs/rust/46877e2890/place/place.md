File_Code/rust/46877e2890/place/place_after.rs --- Rust
588             StaticKind::Promoted(promoted, _) => {                                                                                                       588             StaticKind::Promoted(promoted, promoted_substs) => {
...                                                                                                                                                          589                 let substs = self.subst_from_frame_and_normalize_erasing_regions(promoted_substs);
589                 let instance = self.frame().instance;                                                                                                    590                 let instance = ty::Instance::new(place_static.def_id, substs);

