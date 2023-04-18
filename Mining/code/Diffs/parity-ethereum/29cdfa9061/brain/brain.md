File_Code/parity-ethereum/29cdfa9061/brain/brain_after.rs --- 1/2 --- Rust
18 use super::{KeyPair, Error, Generator, Secret};                                                                                                           18 use super::{KeyPair, Error, Generator};

File_Code/parity-ethereum/29cdfa9061/brain/brain_after.rs --- 2/2 --- Rust
41                                         let result = KeyPair::from_secret(Secret::from(secret.clone()));                                                  41                                         let result = KeyPair::from_secret(secret.clone().into());
42                                         if result.is_ok() {                                                                                               42                                         if result.as_ref().ok().map_or(false, |r| r.address()[0] == 0) {
43                                                 return result                                                                                             43                                                 return result;

