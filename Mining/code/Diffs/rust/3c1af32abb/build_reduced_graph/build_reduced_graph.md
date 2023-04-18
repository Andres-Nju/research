File_Code/rust/3c1af32abb/build_reduced_graph/build_reduced_graph_after.rs --- Rust
  .                                                                                                                                                          526         let module =
526         self.arenas.alloc_module(ModuleData::new(parent, kind, def_id, Mark::root(), DUMMY_SP))                                                          527             self.arenas.alloc_module(ModuleData::new(parent, kind, def_id, Mark::root(), DUMMY_SP));
                                                                                                                                                             528         self.extern_module_map.insert((def_id, macros_only), module);
                                                                                                                                                             529         module

