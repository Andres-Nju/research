File_Code/parity-ethereum/94cb3b6e0e/mod/mod_after.rs --- Rust
535                                 peer_info.status.head_num >= announcement.head_num ||                                                                    535                                 peer_info.status.head_num >= announcement.head_num ||
                                                                                                                                                             536                                 // fix for underflow reported in
                                                                                                                                                             537                                 // https://github.com/paritytech/parity-ethereum/issues/10419
                                                                                                                                                             538                                 now < peer_info.last_update ||

