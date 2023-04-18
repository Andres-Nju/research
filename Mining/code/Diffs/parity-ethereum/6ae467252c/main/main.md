File_Code/parity-ethereum/6ae467252c/main/main_after.rs --- 1/3 --- Rust
187         let logger = setup_log::setup_log(&conf.args.flag_logging, !conf.args.flag_no_color);                                                            187         let logger = setup_log::setup_log(&conf.args.flag_logging, conf.have_color());

File_Code/parity-ethereum/6ae467252c/main/main_after.rs --- 2/3 --- Rust
326         let _logger = setup_log::setup_log(&conf.args.flag_logging, conf.args.flag_no_color);                                                            326         let _logger = setup_log::setup_log(&conf.args.flag_logging, conf.have_color());

File_Code/parity-ethereum/6ae467252c/main/main_after.rs --- 3/3 --- Rust
400         let _logger = setup_log::setup_log(&conf.args.flag_logging, conf.args.flag_no_color);                                                            400         let _logger = setup_log::setup_log(&conf.args.flag_logging, conf.have_color());

