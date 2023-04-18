File_Code/parity-ethereum/94e3a98524/informant/informant_after.rs --- Rust
197                                 let (skipped, skipped_txs) = (self.skipped.load(AtomicOrdering::Relaxed) + imported.len() - 1, self.skipped.load(AtomicO 197                                 let (skipped, skipped_txs) = (self.skipped.load(AtomicOrdering::Relaxed) + imported.len() - 1, self.skipped_txs.load(Ato
    rdering::Relaxed) + txs_imported);                                                                                                                           micOrdering::Relaxed) + txs_imported);

