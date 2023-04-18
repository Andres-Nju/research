File_Code/rust/36ba8ee444/hir_stats/hir_stats_after.rs --- Rust
                                                                                                                                                           128     fn visit_nested_body(&mut self, body_id: hir::BodyId) {
                                                                                                                                                           129         let nested_body = self.krate.unwrap().body(body_id);
                                                                                                                                                           130         self.visit_body(nested_body)
                                                                                                                                                           131     }

