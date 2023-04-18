File_Code/parity-ethereum/9e88acb8c8/bigint/bigint_after.rs --- Rust
27 extern crate ethcore_util;                                                                                                                                27 extern crate bigint;
28 extern crate rand;                                                                                                                                        28 extern crate rand;
29                                                                                                                                                           29 
30 use test::{Bencher, black_box};                                                                                                                           30 use test::{Bencher, black_box};
31 use ethcore_util::U256;                                                                                                                                   31 use bigint::uint::{U256, U512, Uint, U128};

