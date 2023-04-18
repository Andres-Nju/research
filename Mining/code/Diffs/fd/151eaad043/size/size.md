File_Code/fd/151eaad043/size/size_after.rs --- Rust
5     static ref SIZE_CAPTURES: Regex = { Regex::new(r"(?i)^([+-])(\d+)(b|[kmgt]i?b?)$").unwrap() };                                                         5     static ref SIZE_CAPTURES: Regex = Regex::new(r"(?i)^([+-])(\d+)(b|[kmgt]i?b?)$").unwrap();

