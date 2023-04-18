File_Code/servo/0235bf2c4d/opts/opts_after.rs --- 1/3 --- Rust
313     pub webrender_batch: bool,                                                                                                                           313     pub webrender_disable_batch: bool,

File_Code/servo/0235bf2c4d/opts/opts_after.rs --- 2/3 --- Rust
362                 "wr-no-batch" => self.webrender_batch = false,                                                                                           362                 "wr-no-batch" => self.webrender_disable_batch = true,

File_Code/servo/0235bf2c4d/opts/opts_after.rs --- 3/3 --- Rust
830         webrender_batch: debug_options.webrender_batch,                                                                                                  830         webrender_batch: !debug_options.webrender_disable_batch,

