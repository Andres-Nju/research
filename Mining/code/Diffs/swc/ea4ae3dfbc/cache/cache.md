File_Code/swc/ea4ae3dfbc/cache/cache_after.rs --- Rust
 .                                                                                                                                                           15 #[cfg(not(target_arch = "wasm32"))]
15 use wasmer::{BaseTunables, CpuFeature, Engine, Module, Store, Target, Triple};                                                                            16 use wasmer::{BaseTunables, CpuFeature, Engine, Target, Triple};
16 #[cfg(all(not(target_arch = "wasm32"), feature = "filesystem_cache"))]                                                                                    17 use wasmer::{Module, Store};

