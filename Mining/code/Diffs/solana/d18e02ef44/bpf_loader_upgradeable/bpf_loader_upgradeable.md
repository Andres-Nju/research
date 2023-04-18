File_Code/solana/d18e02ef44/bpf_loader_upgradeable/bpf_loader_upgradeable_after.rs --- 1/3 --- Rust
192     3 == instruction_data[0]                                                                                                                             192     !instruction_data.is_empty() && 3 == instruction_data[0]
193 }                                                                                                                                                        193 }
194                                                                                                                                                          194 
195 pub fn is_set_authority_instruction(instruction_data: &[u8]) -> bool {                                                                                   195 pub fn is_set_authority_instruction(instruction_data: &[u8]) -> bool {
196     4 == instruction_data[0]                                                                                                                             196     !instruction_data.is_empty() && 4 == instruction_data[0]

File_Code/solana/d18e02ef44/bpf_loader_upgradeable/bpf_loader_upgradeable_after.rs --- 2/3 --- Rust
                                                                                                                                                             334         assert!(!is_set_authority_instruction(&[]));

File_Code/solana/d18e02ef44/bpf_loader_upgradeable/bpf_loader_upgradeable_after.rs --- 3/3 --- Rust
                                                                                                                                                             343         assert!(!is_upgrade_instruction(&[]));

