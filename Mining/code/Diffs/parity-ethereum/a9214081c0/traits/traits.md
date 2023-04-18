File_Code/parity-ethereum/a9214081c0/traits/traits_after.rs --- 1/2 --- Rust
17 use std;                                                                                                                                                    
18 use std::error::Error as StdError;                                                                                                                          

File_Code/parity-ethereum/a9214081c0/traits/traits_after.rs --- 2/2 --- Rust
33                 Error::Io(err.description().to_owned())                                                                                                   31                 Error::Io(err.to_string())

