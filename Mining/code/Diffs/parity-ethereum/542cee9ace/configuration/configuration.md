File_Code/parity-ethereum/542cee9ace/configuration/configuration_after.rs --- Rust
654                 }                                                                                                                                        654                 } else if self.chain()? != SpecType::Foundation {
                                                                                                                                                             655                         return Ok(GasPricerConfig::Fixed(U256::zero()));
                                                                                                                                                             656                 }

