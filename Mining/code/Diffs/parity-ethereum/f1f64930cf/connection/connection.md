File_Code/parity-ethereum/f1f64930cf/connection/connection_after.rs --- 1/2 --- Rust
223                 if self.registered.load(AtomicOrdering::SeqCst) {                                                                                        223                 if self.registered.compare_and_swap(false, true, AtomicOrdering::SeqCst) {

File_Code/parity-ethereum/f1f64930cf/connection/connection_after.rs --- 2/2 --- Rust
230                 self.registered.store(true, AtomicOrdering::SeqCst);                                                                                         

