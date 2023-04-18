File_Code/deno/1ac80feb1c/ops/ops_after.rs --- 1/2 --- Rust
5 use errors::DenoError;                                                                                                                                     5 use errors::{DenoError, DenoResult, ErrorKind};
6 use errors::DenoResult;                                                                                                                                      

File_Code/deno/1ac80feb1c/ops/ops_after.rs --- 2/2 --- Rust
970     panic!("symlink for windows is not yet implemented")                                                                                                 969     return odd_future(errors::new(
                                                                                                                                                             970       ErrorKind::Other,
                                                                                                                                                             971       "symlink for windows is not yet implemented".to_string(),
                                                                                                                                                             972     ));

