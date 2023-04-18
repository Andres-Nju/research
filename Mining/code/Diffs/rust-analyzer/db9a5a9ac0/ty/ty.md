File_Code/rust-analyzer/db9a5a9ac0/ty/ty_after.rs --- 1/2 --- Rust
446         assert_eq!(substs.len(), def_generics.params.len());                                                                                             446         assert_eq!(substs.len(), def_generics.count_params_including_parent());

File_Code/rust-analyzer/db9a5a9ac0/ty/ty_after.rs --- 2/2 --- Rust
                                                                                                                                                            1377         assert_eq!(substs.len(), parent_param_count + param_count);

