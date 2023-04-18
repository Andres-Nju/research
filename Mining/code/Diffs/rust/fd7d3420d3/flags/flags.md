File_Code/rust/fd7d3420d3/flags/flags_after.rs --- 1/3 --- Rust
451             stage: matches.opt_str("stage").map(|j| j.parse().unwrap()),                                                                                 451             stage: matches.opt_str("stage").map(|j| j.parse().expect("`stage` should be a number")),

File_Code/rust/fd7d3420d3/flags/flags_after.rs --- 2/3 --- Rust
456                 .into_iter().map(|j| j.parse().unwrap())                                                                                                 456                 .into_iter().map(|j| j.parse().expect("`keep-stage` should be a number"))

File_Code/rust/fd7d3420d3/flags/flags_after.rs --- 3/3 --- Rust
467             jobs: matches.opt_str("jobs").map(|j| j.parse().unwrap()),                                                                                   467             jobs: matches.opt_str("jobs").map(|j| j.parse().expect("`jobs` should be a number")),

