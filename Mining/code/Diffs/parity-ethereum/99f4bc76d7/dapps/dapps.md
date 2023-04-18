File_Code/parity-ethereum/99f4bc76d7/dapps/dapps_after.rs --- Rust
191         pub type SyncStatus = Fn() -> bool;                                                                                                              191         pub trait SyncStatus {
                                                                                                                                                             192                 fn is_major_importing(&self) -> bool;
                                                                                                                                                             193                 fn peers(&self) -> (usize, usize);
                                                                                                                                                             194         }

