File_Code/rust/f3993d0619/mod/mod_after.rs --- 1/2 --- Rust
1060         self.all_ids().filter(move |hir| nodes.matces_suffix(*hir)).map(move |hir| {                                                                    1060         self.all_ids().filter(move |hir| nodes.matches_suffix(*hir)).map(move |hir| {

File_Code/rust/f3993d0619/mod/mod_after.rs --- 2/2 --- Rust
1201     fn matces_suffix(&self, hir: HirId) -> bool {                                                                                                       1201     fn matches_suffix(&self, hir: HirId) -> bool {

