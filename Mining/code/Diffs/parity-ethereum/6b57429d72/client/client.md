File_Code/parity-ethereum/6b57429d72/client/client_after.rs --- Rust
2041                 Ok(self.chain.read().logs(blocks, |entry| filter.matches(entry), filter.limit))                                                         2041                 Ok(chain.logs(blocks, |entry| filter.matches(entry), filter.limit))

