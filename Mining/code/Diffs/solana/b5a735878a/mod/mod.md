File_Code/solana/b5a735878a/mod/mod_after.rs --- 1/2 --- Rust
                                                                                                                                                             7     program_error::ProgramError,

File_Code/solana/b5a735878a/mod/mod_after.rs --- 2/2 --- Rust
73     fn from_account_info(account_info: &AccountInfo) -> Option<Self> {                                                                                    74     fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
74         bincode::deserialize(&account_info.data.borrow()).ok()                                                                                            75         bincode::deserialize(&account_info.data.borrow()).map_err(|_| ProgramError::InvalidArgument)

