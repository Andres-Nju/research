File_Code/rust/1e0043eb6c/vec/vec_after.rs --- Rust
596     /// owned by the vector were not freed prior to the `set_len` call:                                                                                  596     /// owned by the inner vectors were not freed prior to the `set_len` call:
597     ///                                                                                                                                                  597     ///
598     /// ```                                                                                                                                              598     /// ```
599     /// let mut vec = vec!['r', 'u', 's', 't'];                                                                                                          599     /// let mut vec = vec![vec![1, 0, 0],
600     ///                                                                                                                                                  600     ///                    vec![0, 1, 0],
                                                                                                                                                             601     ///                    vec![0, 0, 1]];

