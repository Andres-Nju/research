File_Code/nushell/983de8974b/base/base_after.rs --- Rust
                                                                                                                                                           150         (Boolean(left), Nothing) => CompareValues::Booleans(*left, false),
                                                                                                                                                           151         (Nothing, Boolean(right)) => CompareValues::Booleans(false, *right),

