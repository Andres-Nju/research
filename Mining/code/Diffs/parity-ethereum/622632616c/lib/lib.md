File_Code/parity-ethereum/622632616c/lib/lib_after.rs --- Rust
40                 let extra = if size % 8 > 0  { 1 } else { 0 };                                                                                            40                 let extra = if size % 64 > 0  { 1 } else { 0 };
41                 BitVecJournal {                                                                                                                           41                 BitVecJournal {
42                         elems: vec![0u64; size / 8 + extra],                                                                                              42                         elems: vec![0u64; size / 64 + extra],

