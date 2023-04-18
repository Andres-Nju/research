File_Code/parity-ethereum/9ad71b7baa/mod/mod_after.rs --- Rust
1023                                 pod_storage.insert(key[..].into(), U256::from(&val[..]).into());                                                        1023                                 pod_storage.insert(key[..].into(), rlp::decode::<U256>(&val[..]).expect("Decoded from trie which was encoded from the s
                                                                                                                                                                  ame type; qed").into());

