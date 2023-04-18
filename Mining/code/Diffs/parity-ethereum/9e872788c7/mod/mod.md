File_Code/parity-ethereum/9e872788c7/mod/mod_after.rs --- 1/3 --- Rust
718                 let remaining = self.step.inner.duration_remaining().as_millis();                                                                        718                 let remaining = AsMillis::as_millis(&self.step.inner.duration_remaining());

File_Code/parity-ethereum/9e872788c7/mod/mod_after.rs --- 2/3 --- Rust
728                         while self.step.inner.duration_remaining().as_millis() == 0 {                                                                    728                         while AsMillis::as_millis(&self.step.inner.duration_remaining()) == 0 {

File_Code/parity-ethereum/9e872788c7/mod/mod_after.rs --- 3/3 --- Rust
738                         let next_run_at = self.step.inner.duration_remaining().as_millis() >> 2;                                                         738                         let next_run_at = AsMillis::as_millis(&self.step.inner.duration_remaining()) >> 2;

