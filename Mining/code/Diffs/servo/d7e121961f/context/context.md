File_Code/servo/d7e121961f/context/context_after.rs --- Rust
                                                                                                                                                            98                 // Disable the style sharing cache on opt builds until
                                                                                                                                                            99                 // bug 1358693 is fixed, but keep it on debug builds to make
                                                                                                                                                           100                 // sure we don't introduce correctness bugs.
                                                                                                                                                           101                 if cfg!(debug_assertions) { get_env("DISABLE_STYLE_SHARING_CACHE") } else { true },

