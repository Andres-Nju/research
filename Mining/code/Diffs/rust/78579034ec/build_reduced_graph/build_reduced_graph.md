File_Code/rust/78579034ec/build_reduced_graph/build_reduced_graph_after.rs --- 1/2 --- Rust
                                                                                                                                                           608         if let TraitItemKind::Macro(_) = item.node {
                                                                                                                                                           609             return self.visit_invoc(item.id);
                                                                                                                                                           610         }

File_Code/rust/78579034ec/build_reduced_graph/build_reduced_graph_after.rs --- 2/2 --- Rust
618             TraitItemKind::Macro(_) => return self.visit_invoc(item.id),                                                                                 622             TraitItemKind::Macro(_) => bug!(),  // handled above

