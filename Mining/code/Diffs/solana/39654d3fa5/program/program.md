File_Code/solana/39654d3fa5/program/program_after.rs --- 1/2 --- Rust
156                                 "Executable program's address, must be a signer for initial deploys, can be a pubkey for upgrades \                      156                                 "Executable program's address, must be a keypair for initial deploys, can be a pubkey for upgrades \
157                                 [default: address of keypair at /path/to/program-keypair.json if present, otherwise a random address]"),                 157                                 [default: address of keypair at /path/to/program-keypair.json if present, otherwise a random address]"),

File_Code/solana/39654d3fa5/program/program_after.rs --- 2/2 --- Rust
                                                                                                                                                             892         if program_signer.is_none() {
                                                                                                                                                             893             return Err(
                                                                                                                                                             894                 "Initial deployments require a keypair be provided for the program id".into(),
                                                                                                                                                             895             );
                                                                                                                                                             896         }

