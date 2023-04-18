File_Code/parity-ethereum/0d2993e46d/mod/mod_after.rs --- 1/2 --- Rust
245         fn verify_block_family(&self, _header: &M::Header, _parent: &M::Header) -> Result<(), Error> { Ok(()) }                                          245         fn verify_block_family(&self, _header: &M::Header, _parent: &M::Header) -> Result<(), M::Error> { Ok(()) }
246                                                                                                                                                          246 
247         /// Phase 4 verification. Verify block header against potentially external data.                                                                 247         /// Phase 4 verification. Verify block header against potentially external data.
248         /// Should only be called when `register_client` has been called previously.                                                                     248         /// Should only be called when `register_client` has been called previously.
249         fn verify_block_external(&self, _header: &M::Header) -> Result<(), Error> { Ok(()) }                                                             249         fn verify_block_external(&self, _header: &M::Header) -> Result<(), M::Error> { Ok(()) }

File_Code/parity-ethereum/0d2993e46d/mod/mod_after.rs --- 2/2 --- Rust
307         fn sign(&self, _hash: H256) -> Result<Signature, Error> { unimplemented!() }                                                                     307         fn sign(&self, _hash: H256) -> Result<Signature, M::Error> { unimplemented!() }

