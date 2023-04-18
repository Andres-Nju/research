File_Code/parity-ethereum/364a1fa5a8/mod/mod_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                            21 use std::cmp;

File_Code/parity-ethereum/364a1fa5a8/mod/mod_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
369                                 let rng = rand::random::<usize>() % num_peers;                                                                           370                                 let rng = rand::random::<usize>() % cmp::max(num_peers, 1);

