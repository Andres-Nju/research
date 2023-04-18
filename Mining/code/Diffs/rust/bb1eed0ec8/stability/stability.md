File_Code/rust/bb1eed0ec8/stability/stability_after.rs --- 1/3 --- Rust
353             hir::ItemKind::Mod(..) => self.check_missing_stability(i.id, i.span, "module"),                                                                

File_Code/rust/bb1eed0ec8/stability/stability_after.rs --- 2/3 --- Rust
362         self.check_missing_stability(ti.id, ti.span, "node");                                                                                            360         self.check_missing_stability(ti.id, ti.span, "item");

File_Code/rust/bb1eed0ec8/stability/stability_after.rs --- 3/3 --- Rust
369             self.check_missing_stability(ii.id, ii.span, "node");                                                                                        367             self.check_missing_stability(ii.id, ii.span, "item");

