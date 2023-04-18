File_Code/cargo/967f0944a1/mod/mod_after.rs --- Rust
763                     "cannot compile `{}` package, because target `{}` \                                                                                  763                     "cannot compile `{}` package, because target `{}` \
764                      does not support the `{}` crate types",                                                                                             764                      does not support the `{}` crate type{}",
765                     unit.pkg,                                                                                                                            765                     unit.pkg,
766                     self.target_triple(),                                                                                                                766                     self.target_triple(),
767                     unsupported.join(", ")                                                                                                               767                     unsupported.join(", "),
                                                                                                                                                             768                     if unsupported.len() == 1 { "" } else { "s" }

