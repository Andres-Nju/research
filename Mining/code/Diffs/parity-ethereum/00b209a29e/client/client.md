File_Code/parity-ethereum/00b209a29e/client/client_after.rs --- Rust
2184                 self.importer.miner.chain_new_blocks(self, &[h.clone()], &[], route.enacted(), route.retracted(), true);                                2184                 self.importer.miner.chain_new_blocks(self, &[h.clone()], &[], route.enacted(), route.retracted(), self.engine.seals_internally().is_som
                                                                                                                                                                  e());

